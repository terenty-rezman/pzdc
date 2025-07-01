use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use tokio::sync::mpsc;

struct Button {
    name: String,
    state: i32,
    dataref: String,
    tcp_send: tokio::sync::mpsc::Sender<String>,
}

trait AsButton {
    fn as_button(&self) -> &Button;
    fn as_button_mut(&mut self) -> &mut Button;
}

#[async_trait]
trait ButtonTrait: AsButton + Send + Sync {
    fn get_state(&self) -> i32 {
        let btn = self.as_button();
        btn.state
    }

    fn name(&self) -> &str {
        let btn = self.as_button();
        &btn.name
    }

    async fn set_state(&mut self, state: i32) {
        let btn = self.as_button_mut();
        btn.state = state;

        let command = format!("set {} {}", btn.dataref, btn.state);
        btn.tcp_send.send(command).await.unwrap();
    }
}

impl ButtonTrait for Button {}

impl AsButton for Button {
    fn as_button(&self) -> &Button {
        self
    }

    fn as_button_mut(&mut self) -> &mut Button {
        self
    }
}

macro_rules! quick_impl {
    ($structname: ident, AsButton) => {
        impl AsButton for $structname {
            fn as_button(&self) -> &Button {
                &self.button
            }

            fn as_button_mut(&mut self) -> &mut Button {
                &mut self.button
            }
        }
    };
}

struct RedButton {
    button: Button,
    color: String,
}

quick_impl!(RedButton, AsButton);

#[async_trait]
impl ButtonTrait for RedButton {
    async fn set_state(&mut self, state: i32) {
        ButtonTrait::set_state(&mut self.button, state).await;
        println!("{} button set_state override", self.color);
    }
}

struct GreenButton {
    button: Button,
    color: String,
}

quick_impl!(GreenButton, AsButton);
impl ButtonTrait for GreenButton {}

#[tokio::main]
async fn main() {
    let (tcp_send, mut rx) = mpsc::channel::<String>(32);

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            println!("send tcp {}", message);
        }
    });

    let buttons: Arc<RwLock<[Box<dyn ButtonTrait>; 2]>> = Arc::new(RwLock::new([
        Box::new(RedButton {
            button: Button {
                name: "red_btn".into(),
                state: 0,
                dataref: "red_dataref".into(),
                tcp_send: tcp_send.clone(),
            },
            color: "red".into(),
        }),
        Box::new(GreenButton {
            button: Button {
                name: "green_btn".into(),
                state: 0,
                dataref: "green_ref".into(),
                tcp_send: tcp_send.clone(),
            },
            color: "green".into(),
        }),
    ]));

    let buttons_clone = buttons.clone();
    tokio::spawn(async move {
        let buttons = &*buttons_clone.read().await;
        for b in buttons {
            println!("send state udp {} {}", b.name(), b.get_state());
        }
    });

    let buttons_clone = buttons.clone();
    tokio::spawn(async move {
        let buttons = &mut *buttons_clone.write().await;
        for b in buttons {
            b.set_state(1).await;
        }
    })
    .await
    .unwrap();
}
