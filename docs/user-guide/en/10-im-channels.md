# 10 · IM Channels

This chapter explains how to connect TPA CoWork to Telegram, WeChat, Discord, Slack, Feishu / Lark, QQ Bot, Signal, iMessage, WhatsApp, Google Chat, LINE, and IRC so you can keep using the same Agents, sessions, and approval flow from a phone or team chat.

**In this chapter**

- [10.1 Add and start an account](#101-add-and-start-an-account)
- [10.2 Credentials and connection methods](#102-credentials-and-connection-methods)
- [10.3 Manage sessions from IM](#103-manage-sessions-from-im)
- [10.4 Reply modes and thinking](#104-reply-modes-and-thinking)
- [10.5 Tool approvals and security](#105-tool-approvals-and-security)
- [10.6 Groups, Knowledge Space, and attachments](#106-groups-knowledge-space-and-attachments)
- [10.7 Common commands](#107-common-commands)
- [10.8 Troubleshooting](#108-troubleshooting)

---

## 10.1 Add and start an account

1. Open “Settings” in the bottom-left corner, then choose “IM Channels.”
2. Select a platform, add an account, and enter the credentials required by that platform.
3. Choose the account's default Agent and, where applicable, the allowed DMs, groups, or channels.
4. Save and start the account. When its status changes to “Running,” send the bot a message from the corresponding IM service.

You can configure several accounts for one platform. Account settings control the default Agent, reply mode, thinking visibility, Knowledge Space access, automatic approval, and related behavior. Some platforms also let a group or topic override the Agent.

## 10.2 Credentials and connection methods

| Channel | Main credential or prerequisite | Notes |
| --- | --- | --- |
| Telegram | Bot Token | Uses long polling; no public callback URL is required |
| WeChat | QR-code sign-in | Connects through iLink |
| Discord | Bot Token | Uses the Gateway and supports DMs, groups, and channels |
| Slack | Bot Token + App Token | Socket Mode must be enabled |
| Feishu / Lark | App ID + App Secret | Uses WebSocket events; supports Feishu, Lark, and private domains |
| QQ Bot | App ID + Client Secret | Supports DM, group, and channel contexts |
| Signal | Phone number + linked signal-cli | signal-cli must be available locally |
| iMessage | macOS + imsg CLI | Requires local access to Messages data on macOS |
| WhatsApp | Bridge URL + Token | Requires a compatible external bridge service |
| Google Chat | Service Account | Its webhook must be reachable from the public internet |
| LINE | Channel Token + Secret | Requires a public webhook and signature verification |
| IRC | Nickname, server, and optional NickServ credentials | Connects directly over TCP/TLS |

Enter credentials only in Settings. If a platform reports an expired or unauthorized token, create a replacement and update the existing account.

## 10.3 Manage sessions from IM

An IM chat is attached to only one TPA CoWork session at a time, and a session is handed to only one IM chat. This prevents one reply from being controlled by several chats simultaneously.

- Type `/sessions` in IM to list regular sessions that can be attached, then choose one.
- Type `/session <id>` to attach a specific session or `/session exit` to detach it.
- From an open desktop session, use “Handover to IM” and select the channel, account, and chat. After handover, IM receives the latest completed answer; an answer already in progress arrives when it is ready.
- If another chat takes the same session, the previous chat is detached automatically and receives a notice where supported.

IM channel sessions cannot use Incognito mode because the external chat can still deliver messages asynchronously.

## 10.4 Reply modes and thinking

Each account can use one IM reply mode:

| Mode | Behavior | Best for |
| --- | --- | --- |
| Per round (split) | Sends each round's narration, tool results, and media in generation order; streaming platforms still update progressively | Recommended; preserves a clear timeline |
| Final answer only | Drops pre-tool narration and sends only the last-round answer, followed by media | A quieter, concise chat |
| Streaming preview | Renders the merged response as one growing message | Watching generation live; unsupported channels automatically degrade to Final |

“Show thinking in IM messages” is off by default. When enabled, model reasoning appears as a quoted block before each round. Keep it disabled when other people can see the chat.

## 10.5 Tool approvals and security

When an Agent calls a tool that needs approval, the request is delivered to the current IM chat. Platforms with interactive controls show approval buttons; other platforms provide text choices that you can reply with.

“Auto Approve Tools” is off by default. Enabling it lets tool calls from that account run without manual confirmation, including commands, file operations, and external services. Use it only for a private account and environment you fully control; do not enable it in a multi-user group.

Never paste Bot Tokens, App Secrets, or OAuth credentials into chat messages, logs, or screenshots. Replace credentials only by editing the account under “Settings → IM Channels.”

## 10.6 Groups, Knowledge Space, and attachments

- Account settings can restrict allowed group chats. Depending on the platform, channels, topics, or groups can also select an Agent. If the bot stays silent, confirm that the chat is allowed and whether the bot must be mentioned.
- IM has no Knowledge Space access by default. First opt the account in from desktop Settings; a group must then confirm access with `/kb on`. Use `/kb off` to revoke that group's access.
- Most channels can receive images, audio, video, and documents natively. When a channel cannot send a result type, TPA CoWork falls back to a download link instead of silently dropping it.
- Each platform imposes its own file-size and media-type limits. If an upload fails, send a smaller file or open the generated result from the desktop session.

## 10.7 Common commands

| Command | Purpose |
| --- | --- |
| `/sessions` | List regular sessions available for attachment |
| `/session [<id>\|exit]` | Inspect, attach, or detach the current session |
| `/projects` | List unarchived projects |
| `/project <name>` | Assign the current session to a project |
| `/kb [on\|off]` | Inspect or change Knowledge Space access for the current group |
| `/imreply` | Inspect or change the account's reply mode |
| `/status` | Show runtime status and the attached IM channel |

`/agent` and `/handover` are not used inside IM. Edit the channel account to change its Agent; use `/session` to take over a session from IM.

## 10.8 Troubleshooting

- **The account will not start**: check credentials, bot permissions on the platform, and network access. For webhook platforms, also verify the public URL and signature configuration.
- **The account runs but receives nothing**: make sure the bot belongs to the target chat, required events or Gateway privileges are enabled, and the DM / group allowlist permits it.
- **Text works but attachments do not**: check file permissions, size limits, and `server.publicBaseUrl` for channels that need a public attachment URL. Unsupported types fall back to links.
- **Streaming preview does nothing**: the channel may not support message editing or drafts, so Preview degrades to Final by design.
- **The service is temporarily offline after restart**: the startup watchdog retries recently used accounts. With recovery notices enabled, recently active chats receive an online-again message.
- **An approval button expired**: the approval may have timed out, been handled on desktop, or belonged to a session taken by another chat. Send the request again.

---

## Next steps

- Connect MCP, Hooks, and more skills → [11 · Connect & Extend](11-connect-and-extend.md)
- Review channel permissions and global security settings → [13 · Settings & Security](13-settings-and-security.md)
