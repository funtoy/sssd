
A simple way to let your app support like ./your_app start | stop | status | daemon.

linux里面，app名字不要超过15个字符

```rust
#[tokio::main]
async fn main() {
    sssd::create(your_async_func).await
}

async fn your_async_func() -> anyhow::Result<()> {
    // ...
}
```