# 长轮询生命周期

```mermaid
sequenceDiagram
    participant App as 用户应用
    participant Client as WeixinClient
    participant Monitor as Monitor Loop
    participant API as iLink Bot API
    participant Handler as MessageHandler

    App->>Client: WeixinClient::builder(config)<br/>.on_message(handler).build()
    App->>Client: client.start(initial_sync_buf)
    Client->>Handler: handler.on_start()
    Client->>Monitor: spawn poll loop

    loop 长轮询循环
        Monitor->>API: POST ilink/bot/getupdates<br/>{get_updates_buf, base_info}
        
        alt 成功
            API-->>Monitor: {msgs, get_updates_buf, longpolling_timeout_ms}
            Monitor->>Handler: handler.on_sync_buf_updated(new_buf)
            loop 遍历每条消息
                Monitor->>Monitor: should_process() 过滤
                Monitor->>Handler: handler.on_message(ctx)
            end
        else 超时
            API-->>Monitor: (timeout)
            Note over Monitor: 视为空响应<br/>不增加失败计数
        else errcode == -14
            API-->>Monitor: Session 过期
            Monitor->>Monitor: session_guard.pause()<br/>暂停 1 小时
        else 其他错误
            API-->>Monitor: 错误
            Monitor->>Monitor: 连续失败 < 3 → 等 2s<br/>连续失败 ≥ 3 → 等 30s
        end
    end

    App->>Client: client.shutdown()
    Client->>Handler: handler.on_shutdown()
```

## 关键参数

| 参数 | 值 | 说明 |
|------|-----|------|
| 长轮询超时 | 35s（服务端可动态调整） | `longpolling_timeout_ms` |
| 最大连续失败 | 3 次 | 超过后进入退避 |
| 退避延迟 | 30s | 连续失败 ≥ 3 |
| 重试延迟 | 2s | 连续失败 < 3 |
| Session 暂停 | 1 小时 | errcode == -14 |

## 消息过滤规则

Monitor 在分发给 Handler 前自动过滤：

- 只处理 `message_type == USER (1)` 的消息
- 跳过 `delete_time_ms > 0` 的撤回消息
- 跳过 `message_state == GENERATING (1)` 的未完成消息
