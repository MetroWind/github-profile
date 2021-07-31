use std::str::FromStr;
use std::collections::HashSet;

use crate::error::Error;
use crate::github;

// Define your enum
pub enum Theme { Light, Dark }

// Implement the trait
impl FromStr for Theme {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(rterr!("Invalid theme: {}", s)),
        }
    }
}

pub struct Profile
{
    pub width: f64,
    pub font_size: f64,
    pub top_langs_count: usize,
    pub top_langs_ignored: HashSet<String>,
    pub top_langs_text_width: f64,
    pub theme: Theme,

    top_langs: Vec<(String, u64)>,
}

impl Default for Profile
{
    fn default() -> Self
    {
        let mut ignores = HashSet::new();
        ignores.insert(String::from("HTML"));

        Self {
            width: 600.0,
            font_size: 12.0,
            top_langs_count: 5,
            top_langs_ignored: ignores,
            top_langs_text_width: 150.0,
            theme: Theme::Dark,

            top_langs: Vec::new(),
        }
    }
}

impl Profile
{
    fn colorForeground(&self) -> &str
    {
        match self.theme
        {
            Theme::Light => "black",
            Theme::Dark => "rgb(201, 209, 217)",
        }
    }

    pub async fn getData(&mut self, client: &github::Client) -> Result<(), Error>
    {
        let usage = client.getOverallLangs(client.getRepoCount().await?).await?;
        self.top_langs = github::topLanguages(usage, self.top_langs_count,
                                              &self.top_langs_ignored);
        Ok(())
    }

    pub fn genSvg(&self) -> String
    {
        let mut lines = Vec::new();
        lines.push(format!(r#"<svg version="1.1" baseProfile="full"
     width="{}" height="{}"
     xmlns="http://www.w3.org/2000/svg">"#, self.width, 400));
        lines.push(format!(r#"<style>
text
{{
font-family: monospace;
fill: {};
font-size: {}px;
}}
.LangBar
{{
fill: {};
}}
  </style>"#,
                           self.colorForeground(), self.font_size,
                           self.colorForeground()));

        let max_lang_size: f64 = self.top_langs[0].1 as f64;
        let lang_bar_max_width = 450.0;

        lines.push(format!(
            r#"<text x="0" y="{}" width="100%">Top languages:</text>"#,
            self.font_size * 1.5));
        let y = self.font_size * 1.5;
        for i in 0..self.top_langs.len()
        {
            let (lang, size) = self.top_langs[i].clone();
            lines.push(format!(r#"<text x="{}" y="{}" width="{}"
text-anchor="end" >{}</text>"#,
                               self.width - lang_bar_max_width - 20.0,
                               y + self.font_size * 1.5 * ((i+1) as f64),
                               self.width - lang_bar_max_width - 20.0,
                               lang));
            lines.push(format!(r#"<rect class="LangBar" x="{}" y="{}" width="{}"
height="{}" />"#,
                               self.width - lang_bar_max_width,
                               y + (self.font_size * 1.5 * i as f64)
                               + self.font_size * 0.5,
                               lang_bar_max_width * (size as f64) / max_lang_size,
                               self.font_size));
        }
        let y = y + self.font_size * 1.5 * self.top_langs.len() as f64;
        lines.push(format!(r#"<text x="0" y="{}" style="font-size: {}px">
This is a test.</text>"#,
                           y + self.font_size * 1.5, 8));
        let y = y + self.font_size * 1.5;
        lines.push("</svg>\n".to_owned());
        lines[0] = format!(r#"<svg version="1.1" baseProfile="full"
     width="{}" height="{}"
     xmlns="http://www.w3.org/2000/svg">"#, self.width, y + self.font_size * 1.5);
        lines.join("\n")
    }
}
