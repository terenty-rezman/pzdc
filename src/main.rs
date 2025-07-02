use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock as StdRwLock};
use tokio::{net::tcp, sync::RwLock};

use async_trait::async_trait;
use tokio::sync::mpsc;

struct Button {
    name: String,
    state: i32,
    dataref: String,
    tcp_send: tokio::sync::mpsc::Sender<(String, i32)>,
}

trait AsButton {
    fn as_button(&self) -> &Button;
    fn as_button_mut(&mut self) -> &mut Button;
}

#[async_trait]
trait ButtonTrait: AsButton + Send + Sync {
    fn get_state(&self) -> i32 {
        let btn = self.as_button();
        PARAMS_STATE.get(btn.dataref.as_str())
    }

    fn name(&self) -> &str {
        let btn = self.as_button();
        &btn.dataref
    }

    async fn set_state(&mut self, state: i32) {
        let btn = self.as_button_mut();

        btn.tcp_send
            .send((btn.dataref.clone(), state))
            .await
            .unwrap();
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

impl RedButton {
    fn new(tcp_send: tokio::sync::mpsc::Sender<(String, i32)>) -> Self {
        Self {
            button: Button {
                name: "red_btn".into(),
                state: 0,
                dataref: "red_dataref".into(),
                tcp_send,
            },
            color: "red".into(),
        }
    }
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

impl GreenButton {
    fn new(tcp_send: tokio::sync::mpsc::Sender<(String, i32)>) -> Self {
        Self {
            button: Button {
                name: "green_btn".into(),
                state: 0,
                dataref: "green_dataref".into(),
                tcp_send,
            },
            color: "green".into(),
        }
    }
}

quick_impl!(GreenButton, AsButton);
impl ButtonTrait for GreenButton {}

struct ParamsState {
    params_state: StdRwLock<HashMap<&'static str, f32>>,
}

impl ParamsState {
    fn get(&self, param_name: &str) -> i32 {
        *self.params_state.read().unwrap().get(param_name).unwrap() as i32
    }

    fn set(&self, param_name: &str, value: i32) {
        *self.params_state.write()
        .unwrap()
        .get_mut(param_name)
        .unwrap() = value as f32;
    }
}

static PARAMS_STATE: LazyLock<ParamsState> = LazyLock::new(|| ParamsState {
    params_state: StdRwLock::new(HashMap::from([
        ("red_dataref", 10f32),
        ("green_dataref", 11f32),
    ])),
});

#[tokio::main]
async fn main() {
    let (tcp_send, mut rx) = mpsc::channel::<(String, i32)>(32);

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            println!("send tcp {} {}", message.0, message.1);
            PARAMS_STATE.set(message.0.as_str(), message.1);
        }
    });

    let buttons: Arc<RwLock<HashMap<&str, Box<dyn ButtonTrait>>>> =
        Arc::new(RwLock::new(HashMap::from([
            (
                "red_btn",
                Box::new(RedButton::new(tcp_send.clone())) as Box<dyn ButtonTrait>,
            ),
            ("green_btn", Box::new(GreenButton::new(tcp_send.clone()))),
        ])));

    let buttons_clone = buttons.clone();
    tokio::spawn(async move {
        loop {
            let buttons = &*buttons_clone.read().await;
            for b in buttons.values() {
                println!("send state udp {} {}", b.name(), b.get_state());
            }
        }
    });

    let buttons_clone = buttons.clone();
    tokio::spawn(async move {
        loop {
            let buttons = &mut *buttons_clone.write().await;
            buttons.get_mut("green_btn").unwrap().set_state(0).await;
            buttons.get_mut("red_btn").unwrap().set_state(0).await;
        }
    });

    let buttons_clone = buttons.clone();
    tokio::spawn(async move {
        loop {
            let buttons = &mut *buttons_clone.write().await;
            for b in buttons.values_mut() {
                b.set_state(1).await;
            }
        }
    })
    .await
    .unwrap();
}
