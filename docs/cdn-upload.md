# CDN 上传流程

```mermaid
sequenceDiagram
    participant Caller as 调用方
    participant SDK as weixin-agent
    participant API as iLink Bot API
    participant CDN as 微信 CDN

    Caller->>SDK: reply_media(file_path) 或 send_media(to, path)
    SDK->>SDK: 检测 MIME 类型<br/>video/* → Video / image/* → Image / 其他 → File
    SDK->>SDK: 读取文件，计算 rawsize + MD5
    SDK->>SDK: 生成 16 字节随机 AES key
    SDK->>SDK: AES-128-ECB 加密（PKCS7 填充）
    SDK->>API: POST ilink/bot/getuploadurl<br/>{filekey, media_type, rawsize, rawfilemd5,<br/>filesize, aeskey(hex), base_info}
    API-->>SDK: {upload_param, upload_full_url}

    alt upload_full_url 存在
        SDK->>CDN: POST upload_full_url<br/>Content-Type: application/octet-stream
    else 构建 URL
        SDK->>CDN: POST cdn_base_url/upload?<br/>encrypted_query_param=...&filekey=...
    end

    loop 最多 3 次重试
        CDN-->>SDK: 200 OK + x-encrypted-param header
        Note over SDK: 4xx → 立即中止<br/>5xx → 重试
    end

    SDK->>SDK: 构建 CdnMedia 引用<br/>{encrypt_query_param, aes_key(base64), encrypt_type: 1}
    SDK->>API: POST ilink/bot/sendmessage<br/>(媒体消息项)
    API-->>SDK: OK
    SDK-->>Caller: SendResult {message_id}
```

## AES-128-ECB 加密细节

| 项目 | 值 |
|------|-----|
| 算法 | AES-128-ECB |
| 填充 | PKCS7 |
| Key 长度 | 16 字节（随机生成） |
| Key 传给 API | hex 编码 |
| Key 放入消息 | base64(hex_string_bytes) |
| 加密后大小 | `((plaintext_size + 1 + 15) / 16) * 16` |
| filekey | 32 字符随机 hex |

## AES Key 解析（下载解密时）

服务端返回的 AES key 有两种编码格式：

| 格式 | 场景 | 解析方式 |
|------|------|---------|
| base64 → 16 字节 | 图片 | 直接使用 |
| base64 → 32 hex 字符 → 16 字节 | 文件/语音/视频 | hex 解码 |

图片特殊处理：优先使用 `ImageItem.aeskey`（hex 编码），回退到 `media.aes_key`。
