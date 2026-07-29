#!/usr/bin/env bash
set -euo pipefail

# einmo_review_client.sh — a thin TUI/bridge over einmo-review-server
# (EIMP-2, docs/eimp/). Unlike scripts/experimental_reviewer.sh (left
# untouched — reference material and fallback, EIMP-2.md §6), this script
# holds NO review state itself: every case's decision lives on the server,
# in its EinmoReview session. This script only ever iterates the ordered
# list of case ids the server hands back and asks the server for
# everything else (worklist, verified bodies, decisions).
#
# ── PHASE E: read-only (list + view) ────────────────────────────────────
#
# This increment only lists cases and shows their input/output/checked/
# verified panes in vim (read-only, `-M`) — no decisions, no promote/flag/
# retract yet. Those land in later phases (F: flag, G: retract, H: promote,
# I: undo), each wired to the server's corresponding endpoint, per
# docs/eimp/EIMP-2.plan.md.
#
# ── USAGE ─────────────────────────────────────────────────────────────────
#
#     ./einmo_review_client.sh [-s SOCKET] [name-filter]
#
#     -s SOCKET   path to the einmo-review-server unix-domain socket
#                 (default: .einmo-review.sock in the current directory)
#     [name-filter]   substring of a case id (e.g. foop/23)
#
# The server must already be running (`einmo-review-server` or
# `cargo einmo-review-server`) — this script never falls back to a direct
# `einmo` call or to experimental_reviewer.sh. If no server socket (and its
# `<socket>.session` sidecar file) is found, it fails fast with a message
# telling you how to start one.

SOCKET=".einmo-review.sock"

while getopts "s:h" opt; do
    case "$opt" in
        s) SOCKET="$OPTARG" ;;
        h) sed -n '3,30p' "$0"; exit 0 ;;
        *) echo "Try: $0 -h" >&2; exit 2 ;;
    esac
done
shift $((OPTIND - 1))

FILTER="${1:-}"

# --- find the server (no fallback — EIMP-2.md §6) -------------------------
SESSION_FILE="${SOCKET}.session"
if [[ ! -S "$SOCKET" ]]; then
    echo "einmo_review_client: no server socket at $SOCKET" >&2
    echo "  Start one first:  cargo einmo-review-server --socket $SOCKET <suite>" >&2
    echo "  or:                einmo-review-server --socket $SOCKET <suite>" >&2
    exit 1
fi
if [[ ! -f "$SESSION_FILE" ]]; then
    echo "einmo_review_client: socket $SOCKET exists but no session file $SESSION_FILE" >&2
    echo "  (a server is running, but this script cannot find its session id)" >&2
    exit 1
fi
SESSION="$(cat "$SESSION_FILE")"
if [[ -z "$SESSION" ]]; then
    echo "einmo_review_client: session file $SESSION_FILE is empty" >&2
    exit 1
fi

command -v jq >/dev/null 2>&1 || {
    echo "einmo_review_client: jq is required (curl + jq talk to the server's JSON API)" >&2
    exit 1
}

# curl --unix-socket + a fixed dummy host: the socket path routes the
# connection; the URL's host part is never actually resolved.
api() {  # api <method> <path> [json-body]
    local method="$1" path="$2" body="${3:-}"
    if [[ -n "$body" ]]; then
        curl -sS --unix-socket "$SOCKET" -X "$method" \
            -H 'content-type: application/json' -d "$body" \
            "http://einmo-review-client$path"
    else
        curl -sS --unix-socket "$SOCKET" -X "$method" \
            "http://einmo-review-client$path"
    fi
}

# --- private scratch space (SECURITY) -------------------------------------
# Same discipline as experimental_reviewer.sh: everything under our scratch
# dir is signed content under review (verified bodies fetched from the
# server). umask 077 + harden_dir keeps it owner-only; EINMO_REVIEW_CLIENT_DIR
# lets you place it somewhere private of your own.
umask 077

harden_dir() {
    chmod -R go-rwx "$1" 2>/dev/null || true
    chmod 700 "$1" 2>/dev/null || true
    local mode
    mode="$(stat -c '%a' "$1" 2>/dev/null || echo '?')"
    if [[ "$mode" != "700" ]]; then
        echo "einmo_review_client: refusing to use $1 — could not secure it to mode 700 (got $mode)." >&2
        exit 1
    fi
    if [[ -n "$(find "$1" \( -perm /0077 \) -print -quit 2>/dev/null)" ]]; then
        echo "einmo_review_client: refusing to use $1 — something under it is group/other-accessible." >&2
        exit 1
    fi
}

if [[ -n "${EINMO_REVIEW_CLIENT_DIR:-}" ]]; then
    mkdir -p "$EINMO_REVIEW_CLIENT_DIR"
    harden_dir "$EINMO_REVIEW_CLIENT_DIR"
    TMP="$(mktemp -d "$EINMO_REVIEW_CLIENT_DIR/einmo_review_client.XXXXXX")"
else
    TMP="$(mktemp -d -t einmo_review_client.XXXXXX)"
fi
harden_dir "$TMP"

cleanup() {
    if [[ -n "${TMP:-}" && -d "$TMP" ]]; then
        rm -rf "$TMP"
    fi
}
trap cleanup EXIT INT TERM

# --- the one array this script needs: an ordered list of case ids --------
# Everything else (decisions, per-case status) is a question asked of the
# server, never local state (EIMP-2.md §5).
list_json="$(api GET "/einmo/$SESSION/cases")"
mapfile -t ids < <(echo "$list_json" | jq -r \
    --arg filter "$FILTER" \
    '.[] | select($filter == "" or (.id | contains($filter))) | .id')

if (( ${#ids[@]} == 0 )); then
    echo "No cases to review${FILTER:+ matching '$FILTER'}."
    exit 0
fi

echo "einmo_review_client: session $SESSION, ${#ids[@]} case(s)${FILTER:+ matching '$FILTER'}"

vimesc() {  # escape a filename for vim's -c "split <name>" (spaces only)
    printf '%s' "$1" | sed 's/ /\\ /g'
}

idx=0
while (( idx < ${#ids[@]} )); do
    id="${ids[$idx]}"
    n=$(( idx + 1 ))
    echo "── [$n/${#ids[@]}] $id"

    # Fetch this case's detail (per-stage status) and every stage's
    # verified body, each a separate GET — the server's single-flight
    # cache means re-fetching the same artifact across a session is cheap
    # after the first read.
    detail_json="$(api GET "/einmo/$SESSION/cases/$id")"
    stages_line="$(echo "$detail_json" | jq -r \
        '.stages | map("\(.[0]):\(.[1] // "—")") | join(" ")')"
    echo "   $stages_line"

    base="$(basename "$id")"
    declare -A pane=()
    for stage in output checked verified; do
        p="$TMP/$stage--$base"
        if body_json="$(api GET "/einmo/$SESSION/cases/$id/body/$stage")" \
            && echo "$body_json" | jq -e '.sections' >/dev/null 2>&1; then
            echo "$body_json" | jq -r '.sections[] | "=== \(.[0]) ===\n\(.[1])"' > "$p"
        else
            reason="$(echo "${body_json:-}" | jq -r '.error // "unavailable"' 2>/dev/null || echo "unavailable")"
            echo "(( $stage: $reason ))" > "$p"
        fi
        pane[$stage]="$p"
    done

    # Read-only 4-pane view: input (from the suite's own einmo list — not
    # yet fetched over HTTP in this phase, so shown as a placeholder note)
    # plus output/checked/verified from the server. `-M` disables editing
    # entirely — Phase E makes no decisions, so there is nothing to write
    # back yet.
    : > "$TMP/input--$base"
    echo "(( input pane: not yet served over HTTP in this phase — see body/output for what was evaluated ))" \
        > "$TMP/input--$base"

    # vim caps -c/--cmd arguments at 10 (MAX_ARG_CMDS); join every setup
    # command into ONE -c argument (newline-separated, same as sourcing a
    # script) to stay well under that ceiling regardless of how many panes
    # or mappings later phases add.
    vim_cmds="set laststatus=2
let &g:statusline = 'einmo_review_client (read-only) . ]c/[c jump . \\\\d diff here . \\\\D diff all . :qa next'
botright split $(vimesc "$TMP/input--$base")
vertical belowright split $(vimesc "${pane[output]}")
vertical belowright split $(vimesc "${pane[checked]}")
vertical belowright split $(vimesc "${pane[verified]}")
function! EinmoReviewToggleDiffHere()
    if &diff | diffoff | else | diffthis | endif
endfunction
function! EinmoReviewToggleDiffAll()
    let l:cur = winnr()
    let l:any = 0
    for l:w in range(2, winnr('\$'))
        if getwinvar(l:w, '&diff') | let l:any = 1 | break | endif
    endfor
    for l:w in range(2, winnr('\$'))
        execute l:w . 'wincmd w'
        if l:any | diffoff | else | diffthis | endif
    endfor
    execute l:cur . 'wincmd w'
endfunction
nnoremap <silent> \\d :call EinmoReviewToggleDiffHere()<CR>
nnoremap <silent> \\D :call EinmoReviewToggleDiffAll()<CR>"

    vim -M -n -c "$vim_cmds" "${pane[output]}" </dev/tty >/dev/tty 2>&1 || true

    # highest-present stage for this case — the natural default source when
    # flagging without being told otherwise (mirrors
    # source_stage_for_promote's "prefer higher stage" preference, review.rs)
    default_flag_stage="$(echo "$detail_json" | jq -r \
        '([.stages[] | select(.[1] != null) | .[0]] | .[-1]) // empty')"
    # same "highest present" preference restricted to checked/verified — the
    # only two retractable baselines (output is regenerated every run,
    # flagged is a terminal sink; transitions::retract refuses both)
    default_retract_stage="$(echo "$detail_json" | jq -r \
        '([.stages[] | select(.[1] != null) | .[0]] | map(select(. == "checked" or . == "verified")) | .[-1]) // empty')"

    read -r -p "   Enter=next · f=flag · k=kick · q=quit: " ans </dev/tty || ans=""
    case "${ans,,}" in
        f*)
            if [[ -z "$default_flag_stage" ]]; then
                echo "   nothing to flag: no stage currently holds this case"
            else
                read -r -p "   flag $default_flag_stage — reason: " reason </dev/tty || reason=""
                flag_body="$(jq -n --arg reason "$reason" '{reason: $reason}')"
                resp="$(api POST "/einmo/$SESSION/cases/$id/flag/$default_flag_stage" "$flag_body")"
                flag_error="$(echo "${resp:-}" | jq -r '.error // empty' 2>/dev/null || true)"
                if [[ -n "$flag_error" ]]; then
                    echo "   flag failed: $flag_error"
                else
                    echo "   flagged $default_flag_stage/$base"
                fi
            fi
            idx=$(( idx + 1 ))
            ;;
        k*)
            if [[ -z "$default_retract_stage" ]]; then
                echo "   nothing to kick: not currently checked or verified"
            else
                resp="$(api POST "/einmo/$SESSION/cases/$id/retract/$default_retract_stage" "")"
                retract_error="$(echo "${resp:-}" | jq -r '.error // empty' 2>/dev/null || true)"
                if [[ -n "$retract_error" ]]; then
                    echo "   kick failed: $retract_error"
                else
                    echo "   kicked $default_retract_stage/$base (cascades to verified if applicable)"
                fi
            fi
            idx=$(( idx + 1 ))
            ;;
        q*) break ;;
        *) idx=$(( idx + 1 )) ;;
    esac
done

echo "einmo_review_client: done."
