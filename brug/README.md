# Brug

*It's a bridge!*

Brug allows you to transform function calls for a implementation to be turned into RPC like enum's, and offers both generation of facade traits and performer traits

An example speaks louder than 2 words:

```rust
use brug::{Performer, tokio::OneShot};

struct MyStruct;

#[brug::performer]
impl MyStruct {
  fn add(a: usize, b: usize) -> usize {
    a + b
  }
}

async fn main() {
  let (s, r) = OneShot::pair();
  let command = MyStructCommand::Add(1, 2, s);
  let mut my = MyStruct;

  my.perform(command).await;
  assert_eq!(r.receive().await.expect("command got dropped before processed"), 3);
}

// The attribute on MyStruct expands to the following:
pub enum MyStructCommand<T: ::brug::Transport> {
  Add(usize, usize, T::Sender<usize>),
}

#[::brug::async_trait]
impl ::brug::Performer<MyStructCommand> for MyStruct {
  async fn perform(&mut self, command: MyStructCommand) {
    match command {
      MyStructCommand::Add(a, b, resp) => {
        ::brug::Sender::send(resp, self.add(a, b)).await;
      }
    }
  }
}

#[::brug::async_trait]
pub trait MyStructFacade<T: ::brug::Transport> {
  async fn add(&self, a: usize, b: usize) -> usize {
    let (s, r) = T::pair();
    self.handle(MyStructCommand::Add(a, b, s)).await;
    return r.receive().await.expect("add didn't return a value");
  }

  async fn handle(&self, command: MyStructCommand<T>);
}

#[::brug::async_trait]
pub trait MyStructFacadeMut<T: ::brug::Transport> {
  async fn add(&mut self, a: usize, b: usize) -> usize {
    let (s, r) = T::pair();
    self.handle(MyStructCommand::Add(a, b, s)).await;
    return r.receive().await.expect("add didn't return a value");
  }

  async fn handle(&mut self, command: MyStructCommand<T>);
}

#[::brug::async_trait]
impl<T: ::brug::Transport, F: MyStructFacade<T> + Send + Sync> MyStructFacadeMut<T> for F {
  async fn handle(&mut self, command: MyStructCommand<T>) {
    MyStructFacadeMut::handle(self, command).await;
  }
}
```

The Command enum allows you to use an RPC pattern for a given struct, and the Facade's allow you to create an object that functions like the given struct, but is actually using said RPC pattern