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
# ── USAGE ─────────────────────────────────────────────────────────────────
#
#     ./einmo_review_client.sh [-p SUITE] [-s SOCKET] [-n] [name-filter]
#
#     -p SUITE    launch a private einmo-review-server for the given suite
#                 directory (EIMP-1 §S.7a: the TUI-owned private-server
#                 shape). The server binds an unpredictable socket in a
#                 mode-700 scratch dir, is torn down on exit, and its path
#                 is never exposed. Mutually exclusive with -s.
#     -s SOCKET   attach to an already-running standalone server at SOCKET
#                 (default: .einmo-review.sock in the current directory).
#                 Mutually exclusive with -p.
#     -n          only show cases that are new or broken — differ between
#                 output and checked stages (ReviewMode::NewOrBroken).
#     [name-filter]   substring of a case id (e.g. foop/23)
#
# With -p, the server is launched in the background, its socket path is
# captured from the first line of stdout, and a trap kills the server
# process and removes the socket on exit. Without -p, the server must
# already be running — this script never falls back to a direct `einmo`
# call or to experimental_reviewer.sh. If no server socket (and its
# `<socket>.session` sidecar file) is found, it fails fast with a message
# telling you how to start one.
#
# Inside vim, \\d fetches server-side diff hunks (output vs checked) and
# \\D toggles vim's built-in diff mode across all panes.

SOCKET=".einmo-review.sock"
PRIVATE_SUITE=""
NEW_OR_BROKEN=""
SERVER_PID=""

while getopts "p:s:nh" opt; do
    case "$opt" in
        p) PRIVATE_SUITE="$OPTARG" ;;
        s) SOCKET="$OPTARG"; SOCKET_EXPLICIT=1 ;;
        n) NEW_OR_BROKEN=1 ;;
        h) sed -n '3,37p' "$0"; exit 0 ;;
        *) echo "Try: $0 -h" >&2; exit 2 ;;
    esac
done
shift $((OPTIND - 1))

if [[ -n "$PRIVATE_SUITE" && -n "${SOCKET_EXPLICIT:-}" ]]; then
    echo "einmo_review_client: -p (private) and -s (socket) are mutually exclusive" >&2
    exit 2
fi

FILTER="${1:-}"

# --- private server launch (EIMP-1 §S.7a) ----------------------------------
# When -p is given, launch einmo-review-server in background with --private,
# capture the socket path from its first stdout line, wait for it to become
# reachable, and arrange to kill it on exit.
if [[ -n "$PRIVATE_SUITE" ]]; then
    if [[ ! -d "$PRIVATE_SUITE" ]]; then
        echo "einmo_review_client: suite directory not found: $PRIVATE_SUITE" >&2
        exit 1
    fi
    SOCKET_PATH_FILE="$(mktemp -t einmo_sock.XXXXXX)"
    einmo-review-server serve --private "$PRIVATE_SUITE" \
        >"$SOCKET_PATH_FILE" 2>/dev/null &
    SERVER_PID=$!

    # Poll for the socket path (server prints it as first stdout line)
    SOCKET=""
    for _i in $(seq 1 50); do
        if [[ -s "$SOCKET_PATH_FILE" ]]; then
            SOCKET="$(head -1 "$SOCKET_PATH_FILE")"
            break
        fi
        sleep 0.1
    done
    rm -f "$SOCKET_PATH_FILE"

    if [[ -z "$SOCKET" ]]; then
        echo "einmo_review_client: private server did not report a socket path (timed out)" >&2
        kill "$SERVER_PID" 2>/dev/null || true
        exit 1
    fi

    # Wait for the server to actually accept connections
    for _i in $(seq 1 50); do
        if curl -sS --unix-socket "$SOCKET" -o /dev/null \
            "http://einmo-review-client/" 2>/dev/null; then
            break
        fi
        sleep 0.1
    done
fi

# --- find the server (no fallback — EIMP-2.md §6) -------------------------
SESSION_FILE="${SOCKET}.session"
if [[ ! -S "$SOCKET" ]]; then
    echo "einmo_review_client: no server socket at $SOCKET" >&2
    echo "  Start one first:  cargo einmo-review-server --socket $SOCKET <suite>" >&2
    echo "  or:                einmo-review-server serve --socket $SOCKET <suite>" >&2
    echo "  or:                $0 -p <suite>  (launch a private server)" >&2
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

# A socket special file existing (-S above) does not mean a server is still
# listening on it — a killed server (vs. one that shut down cleanly via
# Ctrl-C/SIGTERM through serve_uds's own cleanup) can leave a stale socket
# file behind. Probe with an actual connection before trusting it, the same
# live-vs-stale distinction serve_uds itself makes on the server side.
if ! curl -sS --unix-socket "$SOCKET" -o /dev/null "http://einmo-review-client/einmo/$SESSION/cases" 2>/dev/null; then
    echo "einmo_review_client: socket $SOCKET exists but nothing is listening (stale socket file — the server was likely killed rather than shut down cleanly)" >&2
    echo "  Start one first:  cargo einmo-review-server --socket $SOCKET <suite>" >&2
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
    if [[ -n "${SERVER_PID:-}" ]]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    if [[ -n "${PRIVATE_SUITE:-}" && -n "${SOCKET:-}" ]]; then
        rm -f "$SOCKET" "${SOCKET}.session"
    fi
    if [[ -n "${TMP:-}" && -d "$TMP" ]]; then
        rm -rf "$TMP"
    fi
}
trap cleanup EXIT INT TERM

# --- server-side diff helper (used by vim's \d mapping) --------------------
DIFF_HELPER="$TMP/diff-helper.sh"
cat > "$DIFF_HELPER" << 'DIFFEOF'
#!/usr/bin/env bash
set -euo pipefail
SOCKET="$1" SESSION="$2" CASE_ID="$3" OUTFILE="$4"
json="$(curl -sS --unix-socket "$SOCKET" \
    "http://einmo-review-client/einmo/$SESSION/cases/$CASE_ID/diff/output/checked" \
    2>/dev/null || true)"
if [[ -z "$json" ]]; then
    echo "(( diff unavailable ))" > "$OUTFILE"
else
    echo "$json" | jq -r '
        .sections[] |
        ("=== " + .name + " ==="),
        (.lines[] |
            if .tag == "equal" then "  " + .text
            elif .tag == "removed" then "- " + .text
            elif .tag == "added" then "+ " + .text
            else "" end)
    ' > "$OUTFILE" 2>/dev/null || echo "(( diff parse error ))" > "$OUTFILE"
fi
DIFFEOF
chmod +x "$DIFF_HELPER"

# --- the one array this script needs: an ordered list of case ids --------
# Everything else (decisions, per-case status) is a question asked of the
# server, never local state (EIMP-2.md §5).
list_json="$(api GET "/einmo/$SESSION/cases")"
if [[ -n "$NEW_OR_BROKEN" ]]; then
    mapfile -t ids < <(echo "$list_json" | jq -r \
        --arg filter "$FILTER" \
        '.[] | select(.differing == true) | select($filter == "" or (.id | contains($filter))) | .id')
else
    mapfile -t ids < <(echo "$list_json" | jq -r \
        --arg filter "$FILTER" \
        '.[] | select($filter == "" or (.id | contains($filter))) | .id')
fi

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
    current_decision="$(echo "$detail_json" | jq -r '.decision // empty')"
    [[ -n "$current_decision" ]] && echo "   pending decision: $current_decision (u to undo)"

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
let &g:statusline = 'einmo_review_client . \\\\d server diff . \\\\D vim diff . :qa next'
let g:einmo_diff_helper = '$DIFF_HELPER'
let g:einmo_socket = '$SOCKET'
let g:einmo_session = '$SESSION'
let g:einmo_case_id = '$(printf '%s' "$id" | sed "s/'/'\\\\''/g")'
let g:einmo_diff_out = '$TMP/server-diff--$base'
botright split $(vimesc "$TMP/input--$base")
vertical belowright split $(vimesc "${pane[output]}")
vertical belowright split $(vimesc "${pane[checked]}")
vertical belowright split $(vimesc "${pane[verified]}")
function! EinmoReviewServerDiff()
    call system(printf('bash %s %s %s %s %s',
        \ shellescape(g:einmo_diff_helper),
        \ shellescape(g:einmo_socket),
        \ shellescape(g:einmo_session),
        \ shellescape(g:einmo_case_id),
        \ shellescape(g:einmo_diff_out)))
    execute 'split ' . fnameescape(g:einmo_diff_out)
    setlocal buftype=nofile bufhidden=wipe noswapfile nomodifiable
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
nnoremap <silent> \\d :call EinmoReviewServerDiff()<CR>
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

    read -r -p "   Enter=next · c=checked · v=verified · f=flag · k=kick · u=undo · q=quit: " ans </dev/tty || ans=""
    case "${ans,,}" in
        u*)
            # Revisit: DELETE clears back to "untouched" — GET .../cases/<id>
            # already told us the current decision above, so there is no
            # local array to search or splice (EIMP-2.md §5); a bare DELETE
            # is a no-op if nothing was pending, so this is safe to send
            # unconditionally.
            resp="$(api DELETE "/einmo/$SESSION/cases/$id/decision" "")"
            undo_error="$(echo "${resp:-}" | jq -r '.error // empty' 2>/dev/null || true)"
            if [[ -n "$undo_error" ]]; then
                echo "   undo failed: $undo_error"
            elif [[ -n "$current_decision" ]]; then
                echo "   undone: $base is untouched again"
            else
                echo "   nothing to undo: $base had no pending decision"
            fi
            idx=$(( idx + 1 ))
            ;;
        c*)
            put_body='{"kind":"promote","to":"checked"}'
            resp="$(api PUT "/einmo/$SESSION/cases/$id/decision" "$put_body")"
            put_error="$(echo "${resp:-}" | jq -r '.error // empty' 2>/dev/null || true)"
            if [[ -n "$put_error" ]]; then
                echo "   decision failed: $put_error"
            else
                echo "   decided: promote $base to checked (applied at end of pass)"
            fi
            idx=$(( idx + 1 ))
            ;;
        v*)
            put_body='{"kind":"promote","to":"verified"}'
            resp="$(api PUT "/einmo/$SESSION/cases/$id/decision" "$put_body")"
            put_error="$(echo "${resp:-}" | jq -r '.error // empty' 2>/dev/null || true)"
            if [[ -n "$put_error" ]]; then
                echo "   decision failed: $put_error"
            else
                echo "   decided: promote $base to verified (applied at end of pass)"
            fi
            idx=$(( idx + 1 ))
            ;;
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

# End-of-pass execute: apply every PUT-recorded decision accumulated above
# (promotions and any skips) in one gated call. Flag/retract already
# executed immediately when decided, so this only ever has promotions left
# to apply (EIMP-2.md §3, §5).
plan_json="$(api GET "/einmo/$SESSION/plan")"
plan_count="$(echo "$plan_json" | jq '.actions | length')"

if (( plan_count == 0 )); then
    echo "einmo_review_client: nothing pending to execute."
else
    echo "── pending plan (${plan_count} action(s)) ──"
    echo "$plan_json" | jq -r '.actions[] | "  " + .kind + " " + .id + (if .to then " -> " + .to else "" end)'
    echo "$plan_json" | jq -r '.claims // [] | .[] | "  CLAIMED: \(.id) (\(.remaining_secs)s remaining)"' 2>/dev/null || true

    needs_verified="$(echo "$plan_json" | jq '[.actions[] | select(.kind == "promote" and .to == "verified")] | length > 0')"
    passphrase=""
    if [[ "$needs_verified" == "true" ]]; then
        read -r -s -p "   checked -> verified passphrase (plaintext over the socket — see EIMP-2.md Open Questions): " passphrase </dev/tty || passphrase=""
        echo
    fi

    read -r -p "   type PROMOTE to apply the plan above, anything else to abort: " confirm </dev/tty || confirm=""
    if [[ "$confirm" == "PROMOTE" ]]; then
        if [[ -n "$passphrase" ]]; then
            exec_body="$(jq -n --arg passphrase "$passphrase" '{confirm: "PROMOTE", passphrase: $passphrase}')"
        else
            exec_body='{"confirm":"PROMOTE"}'
        fi
        passphrase=""
        resp="$(api POST "/einmo/$SESSION/execute" "$exec_body")"
        exec_error="$(echo "${resp:-}" | jq -r '.error // empty' 2>/dev/null || true)"
        if [[ -n "$exec_error" ]]; then
            echo "   execute failed: $exec_error"
        else
            executed_count="$(echo "$resp" | jq '.executed | length')"
            skipped_count="$(echo "$resp" | jq '.skipped | length')"
            echo "   executed: $executed_count, skipped (drifted): $skipped_count"
        fi
    else
        echo "   aborted: plan left pending, nothing executed."
    fi
fi

echo "einmo_review_client: done."
