use serde::{Serialize, Deserialize};
use crate::budget::Tier;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    #[serde(rename = "@id")]
    pub id: String,
    pub action: String,
    pub verify: String,
    pub done: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlPlan {
    pub id: String,
    pub title: String,
    pub wave: String,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub skills_required: Vec<String>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    pub budget_tier: Tier,
    pub budget_lines: u32,
    pub commit_message_hint: String,
}

impl XmlPlan {
    pub fn to_xml(&self) -> Result<String, String> {
        use std::fmt::Write as _;
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let _ = writeln!(xml, "<plan id=\"{}\">", escape_xml(&self.id));
        let _ = writeln!(xml, "  <title>{}</title>", escape_xml(&self.title));
        let _ = writeln!(xml, "  <wave>{}</wave>", escape_xml(&self.wave));
        let _ = writeln!(xml, "  <budget tier=\"{}\" lines=\"{}\"/>",
            format!("{:?}", self.budget_tier).to_lowercase(), self.budget_lines);
        let _ = writeln!(xml, "  <commit_message_hint>{}</commit_message_hint>",
            escape_xml(&self.commit_message_hint));
        if !self.depends.is_empty() {
            xml.push_str("  <depends>\n");
            for d in &self.depends { let _ = writeln!(xml, "    <dep>{}</dep>", escape_xml(d)); }
            xml.push_str("  </depends>\n");
        }
        if !self.files.is_empty() {
            xml.push_str("  <files>\n");
            for f in &self.files { let _ = writeln!(xml, "    <file>{}</file>", escape_xml(f)); }
            xml.push_str("  </files>\n");
        }
        if !self.skills_required.is_empty() {
            xml.push_str("  <skills_required>\n");
            for s in &self.skills_required { let _ = writeln!(xml, "    <skill>{}</skill>", escape_xml(s)); }
            xml.push_str("  </skills_required>\n");
        }
        xml.push_str("  <tasks>\n");
        for t in &self.tasks {
            let _ = writeln!(xml, "    <task id=\"{}\">", escape_xml(&t.id));
            let _ = writeln!(xml, "      <action>{}</action>", escape_xml(&t.action));
            let _ = writeln!(xml, "      <verify>{}</verify>", escape_xml(&t.verify));
            let _ = writeln!(xml, "      <done>{}</done>", escape_xml(&t.done));
            xml.push_str("    </task>\n");
        }
        xml.push_str("  </tasks>\n");
        xml.push_str("</plan>\n");
        Ok(xml)
    }

    pub fn from_xml(xml: &str) -> Result<Self, String> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut id = String::new();
        let mut title = String::new();
        let mut wave = String::new();
        let mut depends = Vec::new();
        let mut files = Vec::new();
        let mut skills_required = Vec::new();
        let mut tasks = Vec::new();
        let mut budget_tier = Tier::Default;
        let mut budget_lines = 0u32;
        let mut commit_message_hint = String::new();

        let mut current_text_target = String::new();
        let mut current_task: Option<(String, String, String, String)> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = std::str::from_utf8(e.name().as_ref()).unwrap_or("").to_string();
                    match name.as_str() {
                        "plan" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"id" {
                                    id = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                            }
                        }
                        "task" => {
                            let mut task_id = String::new();
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"id" {
                                    task_id = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                            }
                            current_task = Some((task_id, String::new(), String::new(), String::new()));
                        }
                        _ => {}
                    }
                    current_text_target = name;
                }
                Ok(Event::Empty(e)) => {
                    let ename = e.name();
                    let name = std::str::from_utf8(ename.as_ref()).unwrap_or("");
                    if name == "budget" {
                        for attr in e.attributes().flatten() {
                            let k = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let v = String::from_utf8_lossy(&attr.value).into_owned();
                            match k {
                                "tier" => budget_tier = Tier::parse(&v).unwrap_or(Tier::Default),
                                "lines" => budget_lines = v.parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().into_owned();
                    if let Some(ref mut task) = current_task {
                        match current_text_target.as_str() {
                            "action" => task.1 = text,
                            "verify" => task.2 = text,
                            "done" => task.3 = text,
                            _ => {}
                        }
                    } else {
                        match current_text_target.as_str() {
                            "title" => title = text,
                            "wave" => wave = text,
                            "dep" => depends.push(text),
                            "file" => files.push(text),
                            "skill" => skills_required.push(text),
                            "commit_message_hint" => commit_message_hint = text,
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let ename = e.name();
                    let name = std::str::from_utf8(ename.as_ref()).unwrap_or("");
                    if name == "task" {
                        if let Some((tid, action, verify, done)) = current_task.take() {
                            tasks.push(Task { id: tid, action, verify, done });
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.to_string()),
                _ => {}
            }
        }

        Ok(XmlPlan { id, title, wave, depends, files, skills_required, tasks,
            budget_tier, budget_lines, commit_message_hint })
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
     .replace('"', "&quot;").replace('\'', "&apos;")
}
