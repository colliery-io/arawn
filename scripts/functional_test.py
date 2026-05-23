#!/usr/bin/env python3
"""Functional test driver — sends a prompt to a running arawn server and analyzes the session."""

import asyncio
import json
import sys
import os
import uuid

import websockets

SERVER_URL = "ws://127.0.0.1:3100/ws"
DATA_DIR = os.path.expanduser("~/.arawn")


async def send_rpc(ws, method, params=None):
    """Send a JSON-RPC request and return the result."""
    req_id = str(uuid.uuid4())[:8]
    msg = {"jsonrpc": "2.0", "method": method, "id": req_id, "params": params or {}}
    await ws.send(json.dumps(msg))

    # Read responses until we get our result
    while True:
        raw = await ws.recv()
        data = json.loads(raw)
        if data.get("id") == req_id:
            if "error" in data:
                raise Exception(f"RPC error: {data['error']}")
            return data.get("result")
        # Otherwise it's a streaming event — skip


async def send_and_wait(ws, session_id, prompt):
    """Send a message and wait for the Complete event."""
    req_id = str(uuid.uuid4())[:8]
    msg = {
        "jsonrpc": "2.0",
        "method": "send_message",
        "id": req_id,
        "params": {"session_id": session_id, "content": prompt},
    }
    await ws.send(json.dumps(msg))

    events = []
    while True:
        raw = await ws.recv()
        data = json.loads(raw)

        # JSON-RPC response (ack)
        if data.get("id") == req_id:
            continue

        # Streaming event (notification)
        if "method" in data and data["method"] == "engine_event":
            event = data.get("params", {})
            events.append(event)
            if event.get("type") in ("complete", "error"):
                break

    return events


def load_session_jsonl(session_id):
    """Load the session JSONL from disk."""
    path = os.path.join(
        DATA_DIR, "workstreams", "scratch", session_id, "messages.jsonl"
    )
    if not os.path.exists(path):
        return []
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def analyze(messages, scenario_name):
    """Analyze session messages and print a report."""
    print(f"\n{'='*60}")
    print(f"SCENARIO: {scenario_name}")
    print(f"{'='*60}")
    print(f"Total messages: {len(messages)}")

    # Count by role
    roles = {}
    for m in messages:
        role = m.get("role", "unknown")
        roles[role] = roles.get(role, 0) + 1
    print(f"By role: {roles}")

    # Assistant messages breakdown
    assistant_msgs = [m for m in messages if m.get("role") == "assistant"]
    tool_only = sum(
        1
        for m in assistant_msgs
        if m.get("tool_uses") and not m.get("content", "").strip()
    )
    text_only = sum(
        1
        for m in assistant_msgs
        if not m.get("tool_uses") and m.get("content", "").strip()
    )
    mixed = len(assistant_msgs) - tool_only - text_only
    print(f"Assistant: {len(assistant_msgs)} total, {tool_only} silent tool-only, {text_only} text-only, {mixed} mixed")

    # Narration ratio
    if assistant_msgs:
        narration = (text_only + mixed) / len(assistant_msgs) * 100
        print(f"Narration ratio: {narration:.0f}%")

    # Tool call analysis
    all_tool_calls = []
    for m in assistant_msgs:
        for tu in m.get("tool_uses", []):
            all_tool_calls.append(tu)
    print(f"Total tool calls: {len(all_tool_calls)}")

    # Tool usage by name
    tool_counts = {}
    for tc in all_tool_calls:
        name = tc.get("name", "?")
        tool_counts[name] = tool_counts.get(name, 0) + 1
    if tool_counts:
        print(f"Tool usage: {dict(sorted(tool_counts.items(), key=lambda x: -x[1]))}")

    # Check for repeated failing calls
    tool_results = [m for m in messages if m.get("role") == "tool_result"]
    error_calls = {}
    for i, m in enumerate(messages):
        if m.get("role") == "tool_result" and m.get("is_error"):
            # Find the preceding tool call
            for j in range(i - 1, -1, -1):
                if messages[j].get("role") == "assistant":
                    for tu in messages[j].get("tool_uses", []):
                        if tu.get("id") == m.get("tool_use_id"):
                            key = f"{tu['name']}({json.dumps(tu.get('input', {}), sort_keys=True)[:80]})"
                            error_calls[key] = error_calls.get(key, 0) + 1
                    break

    repeated_failures = {k: v for k, v in error_calls.items() if v >= 2}
    if repeated_failures:
        print(f"WARN: Repeated failing calls: {repeated_failures}")
    else:
        print("OK: No repeated failing calls")

    # Check for grep used on non-local targets
    grep_misuse = [
        tc
        for tc in all_tool_calls
        if tc.get("name") == "grep"
        and (
            "http" in json.dumps(tc.get("input", {})).lower()
            or not tc.get("input", {}).get("path", "").strip()
        )
    ]
    if grep_misuse:
        print(f"WARN: grep misused ({len(grep_misuse)} calls with empty path or URL)")
    else:
        print("OK: grep used correctly")

    # Turns to complete
    print(f"Turns (assistant messages): {len(assistant_msgs)}")

    print()
    return {
        "total_messages": len(messages),
        "assistant_messages": len(assistant_msgs),
        "tool_only": tool_only,
        "narration_ratio": (text_only + mixed) / max(len(assistant_msgs), 1),
        "total_tool_calls": len(all_tool_calls),
        "repeated_failures": len(repeated_failures),
        "grep_misuse": len(grep_misuse),
    }


async def run_scenario(prompt, name="test"):
    """Connect, send prompt, wait, analyze."""
    print(f"Connecting to {SERVER_URL}...")
    async with websockets.connect(SERVER_URL) as ws:
        # Create session
        result = await send_rpc(ws, "create_session", {"workstream_id": None})
        session_id = result["id"]
        print(f"Session: {session_id}")

        # Send prompt
        print(f"Sending: {prompt[:80]}...")
        events = await send_and_wait(ws, session_id, prompt)
        print(f"Received {len(events)} events")

        # Load and analyze
        messages = load_session_jsonl(session_id)
        return analyze(messages, name)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: functional_test.py '<prompt>' [scenario_name]")
        sys.exit(1)

    prompt = sys.argv[1]
    name = sys.argv[2] if len(sys.argv) > 2 else "manual"
    asyncio.run(run_scenario(prompt, name))
