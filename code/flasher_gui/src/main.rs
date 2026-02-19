use iced::widget::{button, center, column, combo_box, scrollable, space, text};
use iced::{Center, Element, Fill, Renderer, Theme, Task};
use rfd::{AsyncFileDialog, FileHandle};

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
}

#[derive(Debug, Clone)]
enum Message {
    Selected(Language),
    OptionHovered(Language),
    BT,
    Button(Origin),
    File(Option<FileHandle>),
    Closed,
}
#[derive(Debug, Clone)]
enum Origin {
    FlashBin,
    Flash,
}

impl Example {
    fn new() -> Self {
        Self {
            languages: combo_box::State::new(Language::ALL.to_vec()),
            selected_language: None,
            text: String::new(),
            flash_file_path: None,
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
                }
            }
            Message::BT => {
                let file_picker = AsyncFileDialog::new()
                            .add_filter("bin", &["bin"])
                            .set_directory(".")
                            .pick_file();
                Task::perform( file_picker, Message::File)
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
        let button_bin_picker: iced::widget::Button<'_, _, Theme, Renderer> = button("pick binary file...").on_press(Message::BT);
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
            button_bin_picker,
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
