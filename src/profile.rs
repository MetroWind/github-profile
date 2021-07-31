use std::collections::HashSet;

use crate::error::Error;
use crate::github;

pub struct Profile
{
    pub width: f64,
    pub font_size: f64,
    pub top_langs_count: usize,
    pub top_langs_ignored: HashSet<String>,
    pub top_langs_text_width: f64,

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
            top_langs: Vec::new(),
        }
    }
}

impl Profile
{
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
fill: currentColor;
font-size: {}px;
}}
.LangBar
{{
fill: currentColor;
}}
  </style>"#,
                           self.font_size));

        let max_lang_size: f64 = self.top_langs[0].1 as f64;
        let lang_bar_max_width = 450.0;
        for i in 0..self.top_langs.len()
        {
            let (lang, size) = self.top_langs[i].clone();
            lines.push(format!(r#"<text x="{}" y="{}" width="{}"
text-anchor="end" >{}</text>"#,
                               self.width - lang_bar_max_width - 20.0,
                               self.font_size * 1.5 * ((i+1) as f64),
                               self.width - lang_bar_max_width - 20.0,
                               lang));
            lines.push(format!(r#"<rect class="LangBar" x="{}" y="{}" width="{}"
height="{}" />"#,
                               self.width - lang_bar_max_width,
                               (self.font_size * 1.5 * i as f64) + 6.0,
                               lang_bar_max_width * (size as f64) / max_lang_size,
                               self.font_size));
        }
        let y = self.font_size * 1.5 * self.top_langs.len() as f64;
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
