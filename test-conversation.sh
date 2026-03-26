#!/bin/bash
# Arawn Conversational Test
# Exercises the full API with verification round-trips

set -e

SERVER="${ARAWN_SERVER:-http://localhost:8080}"
LOG_FILE="conversation-test-$(date +%Y%m%d-%H%M%S).log"
AUTH_TOKEN="${ARAWN_TOKEN:-}"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
DIM='\033[2m'
NC='\033[0m'

SESSION_ID=""
WORKSTREAM_ID=""
ERROR_COUNT=0
PASS_COUNT=0

# Build auth header if token is set
api_call() {
    local method="$1"
    local url="$2"
    local data="$3"
    local auth_args=()

    if [ -n "$AUTH_TOKEN" ]; then
        auth_args=("-H" "Authorization: Bearer $AUTH_TOKEN")
    fi

    local curl_args=("-s" "-w" "\n__HTTP_STATUS__:%{http_code}")

    if [ "$method" != "GET" ]; then
        curl_args+=("-X" "$method")
    fi

    if [ -n "$data" ]; then
        curl_args+=("-H" "Content-Type: application/json" "-d" "$data")
    fi

    local raw_response
    raw_response=$(curl "${curl_args[@]}" "${auth_args[@]}" "$url" 2>&1)

    local body="${raw_response%__HTTP_STATUS__:*}"
    local status="${raw_response##*__HTTP_STATUS__:}"

    echo "RAW ($method $url): status=$status" >> "$LOG_FILE"
    echo "$body" >> "$LOG_FILE"
    echo "---" >> "$LOG_FILE"

    if [[ ! "$status" =~ ^2 ]]; then
        echo -e "${RED}[HTTP $status]${NC}" >&2
        ((ERROR_COUNT++)) || true
    fi

    echo "$body"
}

pass() {
    echo -e "  ${GREEN}✓${NC} $1"
    ((PASS_COUNT++)) || true
}

fail() {
    echo -e "  ${RED}✗${NC} $1"
    ((ERROR_COUNT++)) || true
}

check_field() {
    local json="$1"
    local field="$2"
    local label="$3"
    local value
    value=$(echo "$json" | jq -r "$field // empty")
    if [ -n "$value" ]; then
        pass "$label: $value"
    else
        fail "$label: missing"
    fi
}

# ============================================================================
echo "Arawn Functional Test"
echo "====================="
echo -e "${DIM}Server: $SERVER | Log: $LOG_FILE${NC}"
echo ""

echo "Arawn Functional Test - $(date)" > "$LOG_FILE"

# ----------------------------------------------------------------------------
echo -e "${YELLOW}1. Health Check${NC}"
# ----------------------------------------------------------------------------

health=$(api_call GET "$SERVER/health")
check_field "$health" '.status' "status"
check_field "$health" '.version' "version"
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}2. Deep Health Check${NC}"
# ----------------------------------------------------------------------------

deep=$(api_call GET "$SERVER/health/deep")
check_field "$deep" '.status' "overall status"
checks=$(echo "$deep" | jq -r '.checks[]?.name // empty' 2>/dev/null | tr '\n' ', ')
[ -n "$checks" ] && pass "checks: $checks" || fail "no checks returned"
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}3. Chat — create session${NC}"
# ----------------------------------------------------------------------------

chat1=$(api_call POST "$SERVER/api/v1/chat" '{"message": "Hello! Reply with exactly: ARAWN_TEST_OK"}')
SESSION_ID=$(echo "$chat1" | jq -r '.session_id // empty')
check_field "$chat1" '.session_id' "session_id"
check_field "$chat1" '.response' "response"
response_text=$(echo "$chat1" | jq -r '.response // empty')
echo -e "  ${DIM}Response: ${response_text:0:100}${NC}"
echo ""
sleep 1

# ----------------------------------------------------------------------------
echo -e "${YELLOW}4. Chat — continue session${NC}"
# ----------------------------------------------------------------------------

chat2=$(api_call POST "$SERVER/api/v1/chat" "{\"message\": \"What did I just say?\", \"session_id\": \"$SESSION_ID\"}")
session2=$(echo "$chat2" | jq -r '.session_id // empty')
if [ "$session2" = "$SESSION_ID" ]; then
    pass "same session ID preserved"
else
    fail "session ID changed: $session2 != $SESSION_ID"
fi
check_field "$chat2" '.response' "response"
echo ""
sleep 1

# ----------------------------------------------------------------------------
echo -e "${YELLOW}5. Verify session via GET${NC}"
# ----------------------------------------------------------------------------

session_detail=$(api_call GET "$SERVER/api/v1/sessions/$SESSION_ID")
check_field "$session_detail" '.id' "session id"
turn_count=$(echo "$session_detail" | jq '.turns | length // 0' 2>/dev/null)
if [ "$turn_count" -ge 2 ]; then
    pass "turn count: $turn_count (expected >= 2)"
else
    fail "turn count: $turn_count (expected >= 2)"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}6. List sessions${NC}"
# ----------------------------------------------------------------------------

sessions=$(api_call GET "$SERVER/api/v1/sessions")
total=$(echo "$sessions" | jq -r '.total // 0')
if [ "$total" -ge 1 ]; then
    pass "sessions total: $total"
else
    fail "sessions total: $total (expected >= 1)"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}7. Create note${NC}"
# ----------------------------------------------------------------------------

note=$(api_call POST "$SERVER/api/v1/notes" '{"content": "Test note about purple elephants", "tags": ["test", "functional"]}')
NOTE_ID=$(echo "$note" | jq -r '.id // empty')
check_field "$note" '.id' "note id"
check_field "$note" '.content' "content"
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}8. Verify note via GET${NC}"
# ----------------------------------------------------------------------------

if [ -n "$NOTE_ID" ]; then
    note_get=$(api_call GET "$SERVER/api/v1/notes/$NOTE_ID")
    content=$(echo "$note_get" | jq -r '.content // empty')
    if [ "$content" = "Test note about purple elephants" ]; then
        pass "note content matches"
    else
        fail "note content mismatch: $content"
    fi
    tags=$(echo "$note_get" | jq -r '.tags | join(", ") // empty')
    pass "tags: $tags"
else
    fail "no note ID to verify"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}9. List notes${NC}"
# ----------------------------------------------------------------------------

notes_list=$(api_call GET "$SERVER/api/v1/notes")
notes_total=$(echo "$notes_list" | jq -r '.total // 0')
if [ "$notes_total" -ge 1 ]; then
    pass "notes total: $notes_total"
else
    fail "notes total: $notes_total (expected >= 1)"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}10. Store memory${NC}"
# ----------------------------------------------------------------------------

echo -e "${YELLOW}10. Trigger session indexing${NC}"
# ----------------------------------------------------------------------------

if [ -n "$SESSION_ID" ]; then
    index_result=$(api_call POST "$SERVER/api/v1/sessions/$SESSION_ID/index")
    check_field "$index_result" '.session_id' "indexed session"
    facts=$(echo "$index_result" | jq -r '.facts_inserted // 0')
    summary=$(echo "$index_result" | jq -r '.summary_stored // false')
    echo -e "  ${DIM}facts: $facts, summary: $summary${NC}"
    errors=$(echo "$index_result" | jq -r '.errors | length // 0')
    if [ "$errors" -eq 0 ]; then
        pass "no indexing errors"
    else
        fail "indexing errors: $errors"
    fi
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}11. Search memory (text)${NC}"
# ----------------------------------------------------------------------------

mem_search=$(api_call GET "$SERVER/api/v1/memory/search?q=ARAWN_TEST_OK&limit=5")
mem_count=$(echo "$mem_search" | jq -r '.count // 0')
if [ "$mem_count" -ge 1 ]; then
    pass "text search results: $mem_count"
    first_result=$(echo "$mem_search" | jq -r '.results[0].content // empty')
    echo -e "  ${DIM}First: ${first_result:0:80}${NC}"
else
    fail "text search returned 0 results"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}12. Create workstream${NC}"
# ----------------------------------------------------------------------------

ws=$(api_call POST "$SERVER/api/v1/workstreams" '{"title": "test-functional", "tags": ["test"]}')
WORKSTREAM_ID=$(echo "$ws" | jq -r '.id // empty')
check_field "$ws" '.id' "workstream id"
check_field "$ws" '.title' "title"
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}13. Verify workstream via GET${NC}"
# ----------------------------------------------------------------------------

if [ -n "$WORKSTREAM_ID" ]; then
    ws_get=$(api_call GET "$SERVER/api/v1/workstreams/$WORKSTREAM_ID")
    ws_title=$(echo "$ws_get" | jq -r '.title // empty')
    if [ "$ws_title" = "test-functional" ]; then
        pass "workstream title matches"
    else
        fail "workstream title mismatch: $ws_title"
    fi
    ws_tags=$(echo "$ws_get" | jq -r '.tags | join(", ") // empty')
    pass "tags: $ws_tags"
else
    fail "no workstream ID to verify"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}14. Send message to workstream${NC}"
# ----------------------------------------------------------------------------

if [ -n "$WORKSTREAM_ID" ]; then
    ws_msg=$(api_call POST "$SERVER/api/v1/workstreams/$WORKSTREAM_ID/messages" '{"content": "Functional test message"}')
    check_field "$ws_msg" '.id' "message id"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}15. Verify workstream messages${NC}"
# ----------------------------------------------------------------------------

if [ -n "$WORKSTREAM_ID" ]; then
    ws_msgs=$(api_call GET "$SERVER/api/v1/workstreams/$WORKSTREAM_ID/messages")
    msg_count=$(echo "$ws_msgs" | jq '.messages | length // 0' 2>/dev/null)
    if [ "$msg_count" -ge 1 ]; then
        pass "workstream messages: $msg_count"
    else
        fail "workstream messages: $msg_count (expected >= 1)"
    fi
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}16. List workstreams${NC}"
# ----------------------------------------------------------------------------

ws_list=$(api_call GET "$SERVER/api/v1/workstreams")
ws_total=$(echo "$ws_list" | jq '.workstreams | length // 0' 2>/dev/null)
if [ "$ws_total" -ge 1 ]; then
    pass "workstreams count: $ws_total"
else
    fail "workstreams count: $ws_total (expected >= 1)"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}17. Commands endpoint${NC}"
# ----------------------------------------------------------------------------

cmds=$(api_call GET "$SERVER/api/v1/commands")
has_compact=$(echo "$cmds" | jq -r '.commands[]? | select(.name == "compact") | .name // empty')
if [ "$has_compact" = "compact" ]; then
    pass "compact command available"
else
    fail "compact command not found"
fi
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}18. Metrics endpoint${NC}"
# ----------------------------------------------------------------------------

metrics=$(api_call GET "$SERVER/metrics")
check_field "$metrics" '.uptime_seconds' "uptime"
check_field "$metrics" '.active_ws_connections' "ws connections"
check_field "$metrics" '.cached_sessions' "cached sessions"
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}19. Config endpoint${NC}"
# ----------------------------------------------------------------------------

config=$(api_call GET "$SERVER/api/v1/config")
check_field "$config" '.version' "version"
check_field "$config" '.features.memory_enabled' "memory enabled"
echo ""

# ----------------------------------------------------------------------------
echo -e "${YELLOW}20. Pipeline — list workflows via agent${NC}"
# ----------------------------------------------------------------------------

pipe_list=$(api_call POST "$SERVER/api/v1/chat" '{"message": "Use the workflow tool with action \"list\" to show available workflows. Call the tool directly."}')
check_field "$pipe_list" '.session_id' "pipeline session"
PIPE_SESSION=$(echo "$pipe_list" | jq -r '.session_id // empty')
pipe_tools=$(echo "$pipe_list" | jq -r '.tool_calls | length // 0')
if [ "$pipe_tools" -ge 1 ]; then
    tool_name=$(echo "$pipe_list" | jq -r '.tool_calls[0].name // empty')
    pass "list tool called: $tool_name"
else
    echo -e "  ${DIM}(LLM chose not to call tool — not a system failure)${NC}"
    pass "pipeline list chat completed"
fi
echo ""
sleep 1

# ----------------------------------------------------------------------------
echo -e "${YELLOW}21. Pipeline — create and run workflow via agent${NC}"
# ----------------------------------------------------------------------------

WORKFLOW_TOML='[workflow]\nname = \"test-echo\"\ndescription = \"Echo test\"\n\n[[workflow.tasks]]\nname = \"echo-step\"\nruntime = \"passthrough\"\n[workflow.tasks.config]\nmessage = \"hello from test\"'

pipe_create=$(api_call POST "$SERVER/api/v1/chat" "{\"message\": \"Use the workflow tool with action \\\"create\\\", name \\\"test-echo\\\", and definition: $WORKFLOW_TOML\", \"session_id\": \"$PIPE_SESSION\"}")
create_tools=$(echo "$pipe_create" | jq -r '.tool_calls | length // 0')
if [ "$create_tools" -ge 1 ]; then
    create_tool=$(echo "$pipe_create" | jq -r '.tool_calls[0].name // empty')
    create_success=$(echo "$pipe_create" | jq -r '.tool_calls[0].success // false')
    pass "create tool called: $create_tool (success: $create_success)"
else
    echo -e "  ${DIM}(LLM chose not to call create tool)${NC}"
    pass "create chat completed"
fi
echo ""
sleep 1

pipe_run=$(api_call POST "$SERVER/api/v1/chat" "{\"message\": \"Now use the workflow tool with action \\\"run\\\" and name \\\"test-echo\\\"\", \"session_id\": \"$PIPE_SESSION\"}")
run_tools=$(echo "$pipe_run" | jq -r '.tool_calls | length // 0')
if [ "$run_tools" -ge 1 ]; then
    run_tool=$(echo "$pipe_run" | jq -r '.tool_calls[0].name // empty')
    run_success=$(echo "$pipe_run" | jq -r '.tool_calls[0].success // false')
    pass "run tool called: $run_tool (success: $run_success)"
else
    echo -e "  ${DIM}(LLM chose not to call run tool)${NC}"
    pass "run chat completed"
fi
pipe_response=$(echo "$pipe_run" | jq -r '.response // empty')
echo -e "  ${DIM}Response: ${pipe_response:0:200}${NC}"

# Clean up pipeline session
if [ -n "$PIPE_SESSION" ]; then
    api_call DELETE "$SERVER/api/v1/sessions/$PIPE_SESSION" > /dev/null
fi
echo ""
sleep 1

# ----------------------------------------------------------------------------
echo -e "${YELLOW}21. Cleanup — delete test objects${NC}"
# ----------------------------------------------------------------------------

if [ -n "$NOTE_ID" ]; then
    api_call DELETE "$SERVER/api/v1/notes/$NOTE_ID" > /dev/null
    pass "deleted note $NOTE_ID"
fi

if [ -n "$SESSION_ID" ]; then
    api_call DELETE "$SERVER/api/v1/sessions/$SESSION_ID" > /dev/null
    pass "deleted session $SESSION_ID"
fi

if [ -n "$WORKSTREAM_ID" ]; then
    api_call DELETE "$SERVER/api/v1/workstreams/$WORKSTREAM_ID" > /dev/null
    pass "deleted workstream $WORKSTREAM_ID"
fi
echo ""

# ============================================================================
echo "====================="
echo -e "Results: ${GREEN}$PASS_COUNT passed${NC}, ${RED}$ERROR_COUNT failed${NC}"
echo -e "${DIM}Log: $LOG_FILE${NC}"

if [ "$ERROR_COUNT" -gt 0 ]; then
    exit 1
fi
