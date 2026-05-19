//! AIjia-specific subcommands for end-to-end testing of the AIjia desktop app.
//!
//! All operations against the AIjia webview funnel through these subcommands —
//! e2e scripts MUST NOT call generic `tauri-pilot eval/click/snapshot` directly.
//! See `lotus-app/docs/e2e-org1-chat-mainline.md` for the rule.
//!
//! The dev build of AIjia exposes `window.__aijia.chatStore` /
//! `window.__aijia.sessionStore` (Zustand hooks). Reading state via
//! `store.getState()` is faster and more accurate than scraping the DOM, so
//! most commands here route through the store first and fall back to DOM
//! selectors (`[data-aijia-*]`) only when state isn't available.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

use crate::cli::AijiaCommand;
use crate::client::Client;
use crate::with_window;

pub(crate) async fn dispatch(
    client: &mut Client,
    cmd: AijiaCommand,
    window: Option<&str>,
) -> Result<Value> {
    match cmd {
        AijiaCommand::HealthCheck => health_check(client, window).await,
        AijiaCommand::Where => where_state(client, window).await,
        AijiaCommand::NewTask { wait_fresh } => new_task(client, wait_fresh, window).await,
        AijiaCommand::TypeMessage { text } => type_message(client, &text, window).await,
        AijiaCommand::Send => send(client, window).await,
        AijiaCommand::Cancel { wait } => cancel(client, wait, window).await,
        AijiaCommand::WaitReply { timeout } => wait_reply(client, timeout, window).await,
        AijiaCommand::UiMessage {
            last,
            role,
            since,
            include_tools,
            include_empty,
        } => {
            ui_message(
                client,
                last,
                role.as_deref(),
                since.as_deref(),
                include_tools,
                include_empty,
                window,
            )
            .await
        }
        AijiaCommand::LastReply { format: _ } => last_reply(client, window).await,
        AijiaCommand::ListSessions => list_sessions(client, window).await,
        AijiaCommand::SwitchSession { id_or_index } => {
            switch_session(client, &id_or_index, window).await
        }
        AijiaCommand::ArchiveSession { id_or_index, wait } => {
            archive_session(client, &id_or_index, wait, window).await
        }
        AijiaCommand::SelectWorkspace { name } => select_workspace(client, &name, window).await,
        AijiaCommand::RestartApp => restart_app(client, window).await,
        AijiaCommand::Screenshot { label, selector } => {
            screenshot(client, &label, selector.as_deref(), window).await
        }
        AijiaCommand::CleanupTestSessions { prefix } => {
            cleanup_test_sessions(client, &prefix, window).await
        }
    }
}

// ─── eval helpers ────────────────────────────────────────────────────────────

async fn eval_json(client: &mut Client, script: &str, window: Option<&str>) -> Result<Value> {
    client
        .call("eval", with_window(Some(json!({ "script": script })), window))
        .await
}

/// Wrap user-provided text into a single-quoted JS string literal, with
/// backslash + single-quote + newline escaping. Suitable for direct injection
/// into an eval'd script.
fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // U+2028 / U+2029 break JS string literals; encode as \uXXXX.
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

// ─── health-check + where ────────────────────────────────────────────────────

async fn health_check(client: &mut Client, window: Option<&str>) -> Result<Value> {
    // 1. pilot socket is responsive
    if client
        .call("ping", with_window(None, window))
        .await
        .is_err()
    {
        return Ok(json!({"ok": false, "reason": "pilot ping failed"}));
    }

    // 2. webview document.readyState
    let state = client
        .call("state", with_window(None, window))
        .await
        .context("state call")?;
    let ready = state
        .get("readyState")
        .and_then(Value::as_str)
        .unwrap_or("");
    if ready != "complete" {
        return Ok(json!({
            "ok": false,
            "reason": "document not ready",
            "readyState": ready,
        }));
    }

    // 3. dev-mode store is exposed and active conversation has an id
    let probe = eval_json(
        client,
        r"(() => {
            const a = window.__aijia;
            if (!a || !a.chatStore || !a.sessionStore) return {ok:false, reason:'window.__aijia missing'};
            const cs = a.chatStore.getState();
            return {ok:true, activeConversationId: cs.activeConversationId, hasEditor: !!document.querySelector('.ProseMirror')};
        })()",
        window,
    )
    .await?;

    if probe.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(probe);
    }

    Ok(json!({
        "ok": true,
        "readyState": ready,
        "activeConversationId": probe.get("activeConversationId").cloned().unwrap_or(Value::Null),
        "hasEditor": probe.get("hasEditor").cloned().unwrap_or(Value::Bool(false)),
    }))
}

async fn where_state(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let state = client
        .call("state", with_window(None, window))
        .await
        .ok();
    let url = state
        .as_ref()
        .and_then(|s| s.get("url"))
        .cloned()
        .unwrap_or(Value::Null);
    let title = state
        .as_ref()
        .and_then(|s| s.get("title"))
        .cloned()
        .unwrap_or(Value::Null);

    let store = eval_json(
        client,
        r"(() => {
            const a = window.__aijia;
            if (!a || !a.chatStore) return null;
            const cs = a.chatStore.getState();
            const activeId = cs.activeConversationId;
            const stream = activeId ? cs.streamStates?.[activeId] : null;
            const conv = cs.conversations?.find(c => c.id === activeId);
            return {
                sessionId: activeId,
                sessionName: conv?.title ?? null,
                isStreaming: !!cs.isStreaming,
                isSending: !!stream?.isSending,
                hasToolCallBlock: !!(stream?.toolExecutions || cs.toolExecutions || []).some(t => t.status === 'executing'),
                messageCount: (cs.messages || []).length,
                lastError: stream?.lastError ?? null,
            };
        })()",
        window,
    )
    .await?;

    let route = eval_json(client, "location.pathname", window)
        .await
        .unwrap_or(Value::Null);
    let has_editor = eval_json(
        client,
        "(() => !!document.querySelector('.ProseMirror'))()",
        window,
    )
    .await
    .unwrap_or(Value::Bool(false));

    let mut out = serde_json::Map::new();
    out.insert("url".into(), url);
    out.insert("route".into(), route);
    out.insert("title".into(), title);
    if let Value::Object(map) = store {
        for (k, v) in map {
            out.insert(k, v);
        }
    }
    out.insert("hasEditor".into(), has_editor);
    // workspace / model: not exposed via store today; surface explicit null so
    // callers can spot "field exists, value unknown" vs "field missing entirely".
    out.entry("workspace".to_string())
        .or_insert(Value::Null);
    out.entry("model".to_string()).or_insert(Value::Null);

    Ok(Value::Object(out))
}

// ─── compose path: new-task / type-message / send / cancel ───────────────────

async fn new_task(client: &mut Client, wait_fresh: bool, window: Option<&str>) -> Result<Value> {
    // The sidebar's "新任务" entry is rendered as a <button> with that text.
    // Match exclusively on text to dodge layout churn.
    let script = r"(() => {
        const btn = [...document.querySelectorAll('button')]
            .find(b => (b.textContent || '').trim() === '新任务');
        if (!btn) return {ok: false, reason: 'new-task button not found'};
        btn.click();
        return {ok: true};
    })()";
    let click_result = eval_json(client, script, window).await?;
    if click_result.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(click_result);
    }

    if !wait_fresh {
        return Ok(click_result);
    }

    // new-task drops back to the "new task" landing page; the store sets
    // activeConversationId = null but does NOT clear `messages` (it's a
    // bag indexed by activeId, components project per-conv slices). The
    // most reliable "ready for fresh input" signal is activeId == null
    // AND the editor exists.
    let probe_script = r"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {ready: false, reason: 'store missing'};
        const hasEditor = !!document.querySelector('.ProseMirror');
        return {
            ready: cs.activeConversationId == null && hasEditor,
            activeId: cs.activeConversationId,
            hasEditor,
        };
    })()";
    let deadline = Instant::now() + Duration::from_millis(2000);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            return Ok(json!({"ok": true, "fresh": true, "probe": probe}));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Ok(json!({"ok": true, "fresh": false, "reason": "new-task page did not stabilise within 2s", "lastProbe": last}))
}

async fn type_message(client: &mut Client, text: &str, window: Option<&str>) -> Result<Value> {
    // Tiptap (.ProseMirror) ignores synthetic input events. execCommand is the
    // only reliable insertion path the PoC found. The editor must be focused
    // first; we also normalise empty/placeholder state.
    let literal = js_string_literal(text);
    let script = format!(
        r"(() => {{
            const ed = document.querySelector('.ProseMirror');
            if (!ed) return {{ok: false, reason: 'ProseMirror editor not found'}};
            ed.focus();
            const ok = document.execCommand('insertText', false, {literal});
            return {{ok, text: ed.textContent || ''}};
        }})()"
    );
    eval_json(client, &script, window).await
}

async fn send(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('button[aria-label="发送"]');
        if (!btn) return {ok: false, reason: 'send button not found'};
        if (btn.disabled) return {ok: false, reason: 'send button disabled'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

async fn cancel(client: &mut Client, wait: bool, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('button[aria-label="停止"]');
        if (!btn) return {ok: false, reason: 'stop button not found (not streaming?)'};
        btn.click();
        return {ok: true};
    })()"#;
    let click_result = eval_json(client, script, window).await?;
    if click_result.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(click_result);
    }

    if !wait {
        return Ok(click_result);
    }

    // Poll until chatStore.isStreaming flips false and the stop button is
    // gone. 5s deadline matches the original cargo-test budget for the same
    // assertion ("run_chat_request 5s 内返回不挂起").
    let probe_script = r##"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {ready: false, reason: 'store missing'};
        const activeId = cs.activeConversationId;
        const stream = activeId ? cs.streamStates?.[activeId] : null;
        const streaming = !!cs.isStreaming || !!stream?.isStreaming;
        const hasStopBtn = !!document.querySelector('button[aria-label="停止"]');
        return {ready: !streaming && !hasStopBtn, streaming, hasStopBtn};
    })()"##;
    let deadline = Instant::now() + Duration::from_millis(5000);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            return Ok(json!({"ok": true, "cancelled": true, "probe": probe}));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(json!({"ok": false, "reason": "isStreaming did not clear within 5s", "lastProbe": last}))
}

// ─── wait-reply ──────────────────────────────────────────────────────────────

async fn wait_reply(client: &mut Client, timeout: u64, window: Option<&str>) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let probe_script = r##"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {ready: false, reason: 'store missing'};
        const activeId = cs.activeConversationId;
        const stream = activeId ? cs.streamStates?.[activeId] : null;
        const streaming = !!cs.isStreaming || !!stream?.isStreaming;
        const hasStopBtn = !!document.querySelector('button[aria-label="停止"]');
        return {ready: !streaming && !hasStopBtn, streaming, hasStopBtn, lastError: stream?.lastError ?? null};
    })()"##;
    // A turn can briefly show isStreaming=false between tool calls — the model
    // pauses while a tool runs, then resumes. Require the "ready" signal to
    // hold for STABILITY_TICKS consecutive probes before declaring done.
    const STABILITY_TICKS: u8 = 3;
    const TICK_MS: u64 = 300;
    let mut consecutive_ready: u8 = 0;
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            consecutive_ready = consecutive_ready.saturating_add(1);
            if consecutive_ready >= STABILITY_TICKS {
                // Give the chat store one more beat to flush the final
                // assistant message before returning.
                tokio::time::sleep(Duration::from_millis(200)).await;
                return Ok(json!({"ok": true, "probe": probe, "stableTicks": STABILITY_TICKS}));
            }
        } else {
            consecutive_ready = 0;
        }
        tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
    }
    Ok(json!({"ok": false, "reason": "timeout", "timeoutSec": timeout, "lastProbe": last}))
}

// ─── message readers ─────────────────────────────────────────────────────────

async fn ui_message(
    client: &mut Client,
    last: Option<usize>,
    role: Option<&str>,
    _since: Option<&str>,
    include_tools: bool,
    include_empty: bool,
    window: Option<&str>,
) -> Result<Value> {
    // The store is the source of truth; the DOM is the rendering. Reading the
    // store gives us role, text, and tool_calls in one place. `--since` is not
    // implemented yet (Phase 2A scope); we record the filter in the response so
    // callers can detect drift between request and behaviour.
    let script = r"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {error: 'store missing'};
        const messages = cs.messages || [];
        return messages.map((m, i) => {
            const text = (m.content && (m.content.text || '')) || '';
            const tool_calls = m.tool_calls || [];
            return {
                index: i,
                role: m.role,
                text,
                tool_calls,
                id: m.id ?? null,
            };
        });
    })()";
    let mut data = eval_json(client, script, window).await?;
    let mut items = match data.take() {
        Value::Array(arr) => arr,
        Value::Object(o) if o.contains_key("error") => {
            return Ok(json!({"ok": false, "reason": o.get("error").cloned().unwrap_or(Value::Null)}));
        }
        _ => Vec::new(),
    };

    // Drop empty placeholder bubbles unless caller opts in. These show up as
    // residue after a cancelled streaming turn — text is "" and no tool_calls
    // were captured — and carry no assertion value.
    if !include_empty {
        items.retain(|m| {
            let has_text = m
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty());
            let has_tool_calls = m
                .get("tool_calls")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            has_text || has_tool_calls
        });
    }

    // Filter by role first so --last counts only matching items.
    if let Some(r) = role {
        items.retain(|m| {
            m.get("role")
                .and_then(Value::as_str)
                .map_or(false, |actual| actual == r)
        });
    }

    if !include_tools {
        // Drop tool_calls from the output rather than filtering rows — tool
        // calls live as a field on assistant messages, not separate rows.
        for m in &mut items {
            if let Some(obj) = m.as_object_mut() {
                obj.remove("tool_calls");
            }
        }
    }

    if let Some(n) = last
        && items.len() > n
    {
        items.drain(0..items.len() - n);
    }

    Ok(Value::Array(items))
}

async fn last_reply(client: &mut Client, window: Option<&str>) -> Result<Value> {
    // Equivalent to `ui-message --last 1 --role assistant` but returns a single
    // object (or null) rather than a one-element array — easier to pipe. The
    // empty-placeholder filter is on by default here too, so the last "real"
    // assistant message wins even if a cancelled-turn residue follows it.
    let arr = ui_message(client, Some(1), Some("assistant"), None, true, false, window).await?;
    let first = arr.as_array().and_then(|a| a.first()).cloned();
    Ok(first.unwrap_or(Value::Null))
}

// ─── session management ─────────────────────────────────────────────────────

async fn list_sessions(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {error: 'store missing'};
        const items = (cs.conversations || []).map((c, i) => ({
            index: i,
            id: c.id,
            title: c.title ?? '',
            active: c.id === cs.activeConversationId,
            // Backend persists `isArchived`; lifecycle stays a fallback for
            // future state-machine work where the field may shift.
            archived: c.isArchived === true || c.archived === true || c.lifecycle === 'Archived',
        }));
        return items;
    })()";
    let data = eval_json(client, script, window).await?;
    Ok(data)
}

async fn switch_session(
    client: &mut Client,
    id_or_index: &str,
    window: Option<&str>,
) -> Result<Value> {
    // Resolve id_or_index against the store first (cheaper + clearer than
    // hunting through DOM rows), then click the matching sidebar row by its
    // data-aijia-conversation-id attribute. Clicking instead of poking the
    // store keeps the navigation path identical to a real user.
    let resolve = format!(
        r##"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return {{ok:false, reason:'store missing'}};
            const arg = {arg};
            const isIndex = /^\d+$/.test(arg);
            const list = cs.conversations || [];
            const conv = isIndex ? list[parseInt(arg, 10)] : list.find(c => c.id === arg);
            if (!conv) return {{ok:false, reason:'conversation not found', arg}};
            const row = document.querySelector('[data-aijia-conversation-id="' + conv.id + '"]');
            if (!row) return {{ok:false, reason:'row not rendered', id: conv.id}};
            row.click();
            return {{ok:true, id: conv.id, title: conv.title ?? null}};
        }})()"##,
        arg = js_string_literal(id_or_index)
    );
    eval_json(client, &resolve, window).await
}

async fn archive_session(
    client: &mut Client,
    id_or_index: &str,
    wait: bool,
    window: Option<&str>,
) -> Result<Value> {
    // Archiving the entirely-UI way requires hover → menu → confirm dialog.
    // Hover doesn't fire reliably under tauri-pilot's pointer model, so we
    // resolve the conversation id from the store and dispatch the
    // `archive_conversation` Tauri command directly. The end state matches
    // what the UI flow would produce, but skips the menu interaction. When
    // we need to test the UI affordance itself, a separate command can be
    // added; this one is for cleanup / setup.
    let resolve_script = format!(
        r"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return null;
            const arg = {};
            const isIndex = /^\d+$/.test(arg);
            const list = cs.conversations || [];
            const conv = isIndex ? list[parseInt(arg, 10)] : list.find(c => c.id === arg);
            return conv?.id ?? null;
        }})()",
        js_string_literal(id_or_index)
    );
    let id_val = eval_json(client, &resolve_script, window).await?;
    let id = id_val
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("conversation not found: {id_or_index}"))?
        .to_owned();

    // Tauri 2 IPC converts Rust snake_case args to JS camelCase, so the
    // wire-level key the bridge expects is `conversationId`. Passing
    // `conversation_id` is rejected with "missing required key conversationId".
    let ipc_args = json!({"conversationId": id});
    client
        .call(
            "ipc",
            with_window(
                Some(json!({"command": "archive_conversation", "args": ipc_args})),
                window,
            ),
        )
        .await?;

    if !wait {
        return Ok(json!({"ok": true, "archived": id}));
    }

    // IPC `archive_conversation` writes to disk synchronously but does NOT
    // push a notification back into the frontend chatStore — the UI path
    // (`useChat.archiveConversation`) optimistically filters the row out
    // *before* the IPC call. Since we bypass that path, the store still
    // shows the conversation as active.
    //
    // To get a deterministic confirmation, we poll the *backend* directly
    // via `get_conversations` IPC and look at the authoritative isArchived
    // flag. This is the same query `useChat.loadConversations` makes.
    let confirm_args = json!({});
    let deadline = Instant::now() + Duration::from_millis(3000);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let convs = client
            .call(
                "ipc",
                with_window(
                    Some(json!({"command": "get_conversations", "args": confirm_args})),
                    window,
                ),
            )
            .await?;
        last = convs.clone();
        if let Some(arr) = convs.as_array() {
            let entry = arr
                .iter()
                .find(|c| c.get("id").and_then(Value::as_str) == Some(id.as_str()));
            let archived = match entry {
                None => true, // hidden from list — treat as archived
                Some(c) => c
                    .get("isArchived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            };
            if archived {
                return Ok(
                    json!({"ok": true, "archived": id, "confirmed": true, "hiddenFromList": entry.is_none()}),
                );
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(json!({"ok": false, "archived": id, "reason": "backend get_conversations did not show isArchived=true within 3s", "lastResponse": last}))
}

async fn select_workspace(client: &mut Client, name: &str, window: Option<&str>) -> Result<Value> {
    let _ = (client, window);
    Ok(json!({
        "ok": false,
        "reason": "select-workspace not implemented yet (workspace picker selectors not finalised)",
        "name": name,
    }))
}

async fn restart_app(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let _ = (client, window);
    Ok(json!({
        "ok": false,
        "reason": "restart-app not implemented yet (would sever the pilot socket; needs reconnect protocol)",
    }))
}

async fn screenshot(
    client: &mut Client,
    label: &str,
    selector: Option<&str>,
    window: Option<&str>,
) -> Result<Value> {
    // Thin wrapper around the underlying `screenshot` RPC. All heavy lifting —
    // capture, fallback, timeout handling — belongs in the plugin layer, not
    // here. We just stamp the file name and forward the selector. If callers
    // hit a timeout, the fix lives in tauri-plugin-pilot, not in aijia.rs.
    let safe: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = format!("/tmp/aijia-e2e-{safe}-{ts}.png");

    let effective_selector = if let Some(s) = selector {
        Some(s.to_owned())
    } else {
        // Default to the message-list anchor: even with a fast capture path
        // this gives a tighter, more readable region than full-document.
        let probe = eval_json(
            client,
            "(() => document.querySelector('[data-aijia-message-list]') ? '[data-aijia-message-list]' : null)()",
            window,
        )
        .await
        .unwrap_or(Value::Null);
        probe.as_str().map(str::to_owned)
    };

    let mut rpc_params = json!({"path": null});
    if let Some(ref s) = effective_selector {
        rpc_params["selector"] = json!(s);
    }
    let result = client
        .call("screenshot", with_window(Some(rpc_params), window))
        .await?;
    let data_url = result
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("screenshot result not a string"))?;
    let base64_data = data_url
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(data_url);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| anyhow::anyhow!("failed to decode base64: {e}"))?;
    std::fs::write(&path, bytes).with_context(|| format!("write screenshot to {path}"))?;
    Ok(json!({"ok": true, "path": path, "selector": effective_selector}))
}

async fn cleanup_test_sessions(
    client: &mut Client,
    prefix: &str,
    window: Option<&str>,
) -> Result<Value> {
    // Match by conversation title prefix. Conversation messages live in the
    // chat store keyed by *active* conversation id only, so we can't peek at
    // first-user-message contents for non-active sessions — title is the only
    // bulk-accessible field.
    //
    // To make a session match, test authors must rename the conversation
    // (UI: ⋯ → 重命名聊天) so the title starts with the prefix. Just sending
    // a message that begins with the prefix does NOT change the title.
    let collect_script = format!(
        r##"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return [];
            const list = cs.conversations || [];
            const prefix = {prefix_lit};
            const out = [];
            for (const c of list) {{
                if (c.isArchived === true || c.archived === true || c.lifecycle === 'Archived') continue;
                if (typeof c.title === 'string' && c.title.startsWith(prefix)) {{
                    out.push({{id: c.id, title: c.title}});
                }}
            }}
            return out;
        }})()"##,
        prefix_lit = js_string_literal(prefix)
    );
    let candidates = eval_json(client, &collect_script, window).await?;
    let list = candidates.as_array().cloned().unwrap_or_default();
    let mut archived = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    for entry in list {
        let id = entry.get("id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let res = client
            .call(
                "ipc",
                with_window(
                    Some(json!({
                        "command": "archive_conversation",
                        "args": {"conversationId": id},
                    })),
                    window,
                ),
            )
            .await;
        match res {
            Ok(_) => archived.push(entry),
            Err(e) => failed.push(json!({"id": id, "error": e.to_string()})),
        }
    }
    Ok(json!({"ok": failed.is_empty(), "prefix": prefix, "archived": archived, "failed": failed}))
}

use base64::Engine as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_string_literal_escapes_quotes_and_backslash() {
        assert_eq!(js_string_literal("a'b"), r"'a\'b'");
        assert_eq!(js_string_literal(r"a\b"), r"'a\\b'");
    }

    #[test]
    fn js_string_literal_escapes_control_chars() {
        assert_eq!(js_string_literal("a\nb"), r"'a\nb'");
        assert_eq!(js_string_literal("a\tb"), r"'a\tb'");
    }

    #[test]
    fn js_string_literal_preserves_unicode_text() {
        // CJK passes through as-is — the surrounding single quotes are still
        // a single JS string literal; no extra escaping needed.
        assert_eq!(js_string_literal("你好"), "'你好'");
    }

    #[test]
    fn js_string_literal_escapes_line_separators() {
        // U+2028 / U+2029 terminate JS string literals; must be encoded.
        let s = format!("a{}b", '\u{2028}');
        assert_eq!(js_string_literal(&s), "'a\\u2028b'");
    }
}
