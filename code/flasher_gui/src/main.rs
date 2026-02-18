use iced::event::{self, Event};
use iced::widget::{button, center, column, combo_box, scrollable, space, text};
use iced::{Center, Element, Fill, Renderer, Theme, Subscription};
use rfd::FileDialog;

pub mod bt_util;
use crate::bt_util::{OTAControlResponse, OTAControl};

/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    // Flash
    scan(&adapter_list, false).await.unwrap();

    // Delay to prevent 
    time::sleep(Duration::from_millis(4000)).await;

    // Verify
    scan(&adapter_list, true).await.unwrap();

    Ok(())
}*/

pub fn main() -> iced::Result {
    iced::application(Example::update, Example::view)
        .subscription(Example::subscription)
        .run()
}

struct Example {
    languages: combo_box::State<Language>,
    selected_language: Option<Language>,
    text: String,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Selected(Language),
    OptionHovered(Language),
    BT,
    Closed,
}

impl Example {
    fn new() -> Self {
        Self {
            languages: combo_box::State::new(Language::ALL.to_vec()),
            selected_language: None,
            text: String::new(),
        }
    }
    fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::EventOccurred)
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::BT => {
                self.text = "connn".to_string();

                // let files = AsyncFileDialog::new()
                //             .add_filter("bin", &["bin", "rs"])
                //             .set_directory(".")
                //             .pick_file().await;
                let files = FileDialog::new()
                            .add_filter("bin", &["bin", "rs"])
                            .set_directory(".")
                            .pick_file();
                if let Some(file) = files {
                    println!("{}",file.display());
                }
            }
            Message::Selected(language) => {
                self.selected_language = Some(language);
                self.text = language.hello().to_string();
            }
            Message::OptionHovered(language) => {
                self.text = language.hello().to_string();
            }
            Message::Closed => {
                self.text = self
                    .selected_language
                    .map(|language| language.hello().to_string())
                    .unwrap_or_default();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let bu: iced::widget::Button<'_, _, Theme, Renderer> = button("Search").on_press(Message::BT);
        let combo_box = combo_box(
            &self.languages,
            "Type a language...",
            self.selected_language.as_ref(),
            Message::Selected,
        )
        .on_option_hovered(Message::OptionHovered)
        .on_close(Message::Closed)
        .width(250);

        let content = column![
            bu,
            text(&self.text),
            "What is your language?",
            combo_box,
            space().height(150),
        ]
        .width(Fill)
        .align_x(Center)
        .spacing(10);

        center(scrollable(content)).into()
    }
}

impl Default for Example {
    fn default() -> Self {
        Example::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    Danish,
    #[default]
    English,
    French,
    German,
    Italian,
    Japanese,
    Portuguese,
    Spanish,
    Other,
}

impl Language {
    const ALL: [Language; 9] = [
        Language::Danish,
        Language::English,
        Language::French,
        Language::German,
        Language::Italian,
        Language::Japanese,
        Language::Portuguese,
        Language::Spanish,
        Language::Other,
    ];

    fn hello(&self) -> &str {
        match self {
            Language::Danish => "Halloy!",
            Language::English => "Hello!",
            Language::French => "Salut!",
            Language::German => "Hallo!",
            Language::Italian => "Ciao!",
            Language::Japanese => "こんにちは!",
            Language::Portuguese => "Olá!",
            Language::Spanish => "¡Hola!",
            Language::Other => "... hello?",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Language::Danish => "Danish",
                Language::English => "English",
                Language::French => "French",
                Language::German => "German",
                Language::Italian => "Italian",
                Language::Japanese => "日本語",
                Language::Portuguese => "Portuguese",
                Language::Spanish => "Spanish",
                Language::Other => "Some other language",
            }
        )
    }
}
