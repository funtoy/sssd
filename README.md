# A simple way to let your app support like

```shell
./your_app start [-d] | stop | status | daemon
```

## Examples

```rust
#[tokio::main]
async fn main() {
    sssd::create(your_async_func, Some(stop_callback_func)).await
}

async fn your_async_func() -> anyhow::Result<()> {
    // ...
}

async fn stop_callback_func() -> anyhow::Result<()> {
    // ...
}
```