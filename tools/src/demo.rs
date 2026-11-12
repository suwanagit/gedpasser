//! Build a test bank: the real content, minus the real content of the
//! subjects the demo defines, plus the made-up demo files in
//! `fixtures/demo-science/`. Used by tests and for trying multi-subject
//! features before real content exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Copy `content` to `out`, drop the real content of every subject the demo
/// defines (so the demo doesn't clash with it as real content grows), then
/// overlay `demo`: a file whose name already exists is appended to, any
/// other file is copied. `out` is replaced.
pub fn build(content: &Path, demo: &Path, out: &Path) -> Result<(), String> {
    if out.exists() {
        std::fs::remove_dir_all(out).map_err(|e| format!("cannot clear {}: {e}", out.display()))?;
    }
    copy_dir(content, out)?;
    let demo_skills = read_toml(&demo.join("skills.toml"))?;
    let subjects: HashSet<String> = tables(&demo_skills, "skill").filter_map(|t| str_field(t, "subject")).collect();
    strip_subjects(out, &subjects)?;
    overlay(demo, out)
}

type Table = toml::map::Map<String, toml::Value>;

fn read_toml(path: &Path) -> Result<Table, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_toml(path: &Path, table: &Table) -> Result<(), String> {
    let text = toml::to_string(table).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

fn tables<'a>(t: &'a Table, key: &str) -> impl Iterator<Item = &'a Table> {
    t.get(key).and_then(|v| v.as_array()).into_iter().flatten().filter_map(|v| v.as_table())
}

fn str_field(t: &Table, key: &str) -> Option<String> {
    t.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

/// Keep only the entries of `table[key]` for which `keep` is true.
fn retain(table: &mut Table, key: &str, keep: impl Fn(&Table) -> bool) {
    if let Some(toml::Value::Array(items)) = table.get_mut(key) {
        items.retain(|v| v.as_table().is_none_or(&keep));
    }
}

/// Remove the real skills, principles, questions, cards, passages, stages,
/// study links and reviews of `subjects` from the bank copy at `out`.
fn strip_subjects(out: &Path, subjects: &HashSet<String>) -> Result<(), String> {
    if subjects.is_empty() {
        return Ok(());
    }
    let mut skills = read_toml(&out.join("skills.toml"))?;
    let gone_skills: HashSet<String> = tables(&skills, "skill")
        .filter(|t| str_field(t, "subject").is_some_and(|s| subjects.contains(&s)))
        .filter_map(|t| str_field(t, "id"))
        .collect();
    retain(&mut skills, "skill", |t| str_field(t, "id").is_none_or(|id| !gone_skills.contains(&id)));
    write_toml(&out.join("skills.toml"), &skills)?;

    let mut principles = read_toml(&out.join("principles.toml"))?;
    let gone_principles: HashSet<String> = tables(&principles, "principle")
        .filter(|t| str_field(t, "skill").is_some_and(|s| gone_skills.contains(&s)))
        .filter_map(|t| str_field(t, "id"))
        .collect();
    retain(&mut principles, "principle", |t| str_field(t, "skill").is_none_or(|s| !gone_skills.contains(&s)));
    write_toml(&out.join("principles.toml"), &principles)?;

    let journey = out.join("journey.toml");
    if journey.exists() {
        let mut j = read_toml(&journey)?;
        retain(&mut j, "stage", |t| str_field(t, "subject").is_none_or(|s| !subjects.contains(&s)));
        write_toml(&journey, &j)?;
    }

    let study = out.join("study.toml");
    if study.exists() {
        let mut s = read_toml(&study)?;
        let before = tables(&s, "topic").count();
        retain(&mut s, "topic", |t| str_field(t, "principle").is_none_or(|p| !gone_principles.contains(&p)));
        if tables(&s, "topic").count() != before {
            // The file's review covered the removed topics too.
            s.insert("status".into(), toml::Value::String("draft".into()));
        }
        write_toml(&study, &s)?;
    }

    let remove = |p: PathBuf| -> Result<(), String> {
        if p.is_dir() {
            std::fs::remove_dir_all(&p).map_err(|e| e.to_string())
        } else if p.exists() {
            std::fs::remove_file(&p).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    };
    for sub in subjects {
        remove(out.join(sub))?;
        remove(out.join("stimuli").join(format!("{sub}.toml")))?;
        remove(out.join("reviews").join(format!("stimuli-{sub}.toml")))?;
    }
    for sk in &gone_skills {
        remove(out.join("cards").join(format!("{sk}.toml")))?;
        remove(out.join("reviews").join(format!("{sk}.toml")))?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(from).map_err(|e| format!("cannot read {}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn overlay(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(from).map_err(|e| format!("cannot read {}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            overlay(&path, &dest)?;
        } else if path.extension().is_some_and(|x| x == "toml") {
            let extra = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut text = std::fs::read_to_string(&dest).unwrap_or_default();
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&extra);
            std::fs::write(&dest, text).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
