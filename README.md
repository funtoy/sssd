
A simple way to let your app support like ./your_app start | stop | status | daemon.

```rust
#[actix_web::main]
async fn main() {
    sssd::create(|| your_async_func()).await
}
```