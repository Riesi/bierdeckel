use iced::widget::{button, center, column, combo_box, scrollable, space, text};
use iced::{Center, Element, Fill, Renderer, Theme, Task};
use rfd::{AsyncFileDialog, FileHandle};

use btleplug::Error;
use btleplug::platform::Manager;
use btleplug::platform::Adapter;
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
    iced::run(Example::update, Example::view)
}

struct Example {
    languages: combo_box::State<Language>,
    selected_language: Option<Language>,
    text: String,
    flash_file_path: Option<FileHandle>,
    bt_adapter_list: Option<Vec<Adapter>>,
}

#[derive(Debug, Clone)]
enum Message {
    Selected(Language),
    OptionHovered(Language),
    Button(Origin),
    File(Option<FileHandle>),
    Bluetooth(BTOrigin),
    Closed,
}
#[derive(Debug, Clone)]
enum Origin {
    FlashBin,
    Flash,
    Search,
}

#[derive(Debug, Clone)]
enum BTOrigin {
    SearchResult(Option<Vec<Adapter>>),
}

async fn test() -> Option<Vec<Adapter>> {
    if let Ok(manager) = Manager::new().await{
    
        if let Ok(adapter_list) = btleplug::api::Manager::adapters(&manager).await {
            if !adapter_list.is_empty() {
                return Some(adapter_list)
            }
            eprintln!("No Bluetooth adapters found");
        }
    }
    None
}

impl Example {
    fn new() -> Self {
        Self {
            languages: combo_box::State::new(Language::ALL.to_vec()),
            selected_language: None,
            text: String::new(),
            flash_file_path: None,
            bt_adapter_list: None,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message>  {
        match message {
            Message::Button(ori) => {
                match ori {
                    Origin::Flash => {
                        Task::none()
                    }
                    Origin::FlashBin => {
                        let file_picker = AsyncFileDialog::new()
                                    .add_filter("bin", &["bin"])
                                    .set_directory(".")
                                    .pick_file();
                        Task::perform( file_picker, Message::File)
                    }
                    Origin::Search => {
                        if let None = self.bt_adapter_list {
                            return Task::perform( test(), |av| Message::Bluetooth(BTOrigin::SearchResult(av)))
                        }
                        Task::none()
                    }
                }
            }
            Message::Bluetooth(bt) => {
                match bt {    
                    BTOrigin::SearchResult(av) => {
                        
                        Task::none()
                    }
                }
            }
            Message::File(file) => {
                if let Some(file) = file {
                    let file_path= &file.path().display();
                    println!("{}",file_path);
                    self.text = file_path.to_string();
                    self.flash_file_path = Some(file);
                }
                Task::none()
            }
            Message::Selected(language) => {
                self.selected_language = Some(language);
                self.text = language.hello().to_string();
                Task::none()
            }
            Message::OptionHovered(language) => {
                self.text = language.hello().to_string();
                Task::none()
            }
            Message::Closed => {
                self.text = self
                    .selected_language
                    .map(|language| language.hello().to_string())
                    .unwrap_or_default();
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Buttons
        let button_bin_picker = button("pick binary file...").on_press(Message::Button(Origin::FlashBin));
        let button_search = button("search for coasters...").on_press(Message::Button(Origin::Search));
        let button_connect = button("connect to coaster...").on_press(Message::Button(Origin::Flash));
        let button_flash = button("flash").on_press(Message::Button(Origin::Flash));


        let combo_box = combo_box(
            &self.languages,
            "Type a language...",
            self.selected_language.as_ref(),
            Message::Selected,
        )
        .on_option_hovered(Message::OptionHovered)
        .on_close(Message::Closed)
        .width(250);

        // Layout
        let content = column![
            button_bin_picker,
            button_search,
            button_connect,
            button_flash,
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
    #[default]
    Danish,
    Other,
}

impl Language {
    const ALL: [Language; 2] = [
        Language::Danish,
        Language::Other,
    ];

    fn hello(&self) -> &str {
        match self {
            Language::Danish => "Halloy!",
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
                Language::Other => "Some other language",
            }
        )
    }
}
