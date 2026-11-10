use std::fs::File;
use std::path::{Path, PathBuf};

use docx_rs::{Docx, LineSpacing, Paragraph, Run, RunFonts};
use serde::Deserialize;
use tauri::{AppHandle, Manager};

use crate::external;

#[derive(Deserialize)]
pub struct WordRequest {
	filename: String,
	title: String,
	sections: Vec<WordSection>,
}

#[derive(Deserialize)]
struct WordSection {
	title: String,
	rows: Vec<[String; 2]>,
}

pub fn export(app: &AppHandle, request: WordRequest) -> Result<String, String> {
	let dir = app.path().download_dir().map_err(|error| error.to_string())?;
	let path = free_path(&dir, &safe_name(&request.filename));
	let fonts = RunFonts::new().ascii("Arial").hi_ansi("Arial").east_asia("Arial").cs("Arial");
	let mut document = Docx::new()
		.default_fonts(fonts)
		.default_size(22)
		.default_line_spacing(LineSpacing::new().line(276))
		.add_paragraph(Paragraph::new().line_spacing(LineSpacing::new().after(240)).add_run(Run::new().add_text(request.title).bold().size(34)));
	for section in request.sections {
		if section.title == "Sintomatologia" { document = document.add_paragraph(Paragraph::new()); }
		if !section.title.is_empty() {
			document = document.add_paragraph(Paragraph::new().line_spacing(LineSpacing::new().before(120).after(120)).add_run(Run::new().add_text(section.title).bold().size(28)));
		}
		for [label, value] in section.rows {
			let paragraph = Paragraph::new().line_spacing(LineSpacing::new().after(40))
				.add_run(Run::new().add_text(format!("{}: ", label)).bold())
				.add_run(Run::new().add_text(if value.trim().is_empty() { "-" } else { value.trim() }));
			document = document.add_paragraph(paragraph);
		}
	}
	let file = File::create(&path).map_err(|error| error.to_string())?;
	if let Err(error) = document.build().pack(file) {
		let _ = std::fs::remove_file(&path);
		return Err(error.to_string());
	}
	let path = path.to_string_lossy().into_owned();
	external::open(&path).map_err(|error| format!("Documento salvato in {}. {}", path, error))?;
	Ok(path)
}

fn safe_name(name: &str) -> String {
	let clean: String = name.chars().take(100).map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
	let clean = clean.trim_matches([' ', '.', '_']);
	if clean.is_empty() { "Riepilogo".to_string() } else { clean.to_string() }
}

fn free_path(dir: &Path, name: &str) -> PathBuf {
	let first = dir.join(format!("{}.docx", name));
	if !first.exists() { return first; }
	(2..).map(|number| dir.join(format!("{} ({}).docx", name, number))).find(|path| !path.exists()).unwrap()
}
