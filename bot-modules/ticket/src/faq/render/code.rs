const ALIASES: &[(&str, &str)] = &[
    ("sh", "bash"),
    ("shell", "bash"),
    ("zsh", "bash"),
    ("console", "bash"),
    ("terminal", "bash"),
    ("dockerfile", "docker"),
    ("jsonc", "json"),
    ("json5", "json"),
    ("yml", "yaml"),
    ("htm", "html"),
    ("md", "markdown"),
    ("py", "python"),
    ("rs", "rust"),
    ("ts", "typescript"),
    ("js", "javascript"),
    ("psl", "powershell"),
    ("ps1", "powershell"),
    ("conf", "ini"),
    ("cfg", "ini"),
    ("env", "ini"),
    ("text", ""),
    ("plaintext", ""),
    ("plain", ""),
];

const FENCE: &str = "```";

pub(crate) fn normalize(block: &str) -> String {
    let mut lines = block.lines();

    let Some(open) = lines.next() else {
        return block.to_owned();
    };

    let indent = leading_spaces(open);
    let language = language(open);

    let body = lines.filter(|line| !is_fence(line)).collect::<Vec<_>>();
    let dedent = body
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| leading_spaces(line))
        .min()
        .unwrap_or(indent);

    let mut out = String::with_capacity(block.len());
    out.push_str(FENCE);
    out.push_str(language);
    out.push('\n');

    for line in body {
        out.push_str(
            line.get(dedent..).unwrap_or_else(|| line.trim_start()).trim_end(),
        );
        out.push('\n');
    }

    out.push_str(FENCE);

    if block.ends_with('\n') {
        out.push('\n');
    }

    out
}

pub(crate) fn fence_indented(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut block: Vec<&str> = Vec::new();

    for line in content.lines() {
        let indented = line.starts_with("    ") || line.starts_with('\t');

        match (indented, line.trim().is_empty()) {
            (true, _) => block.push(line),
            (false, true) if !block.is_empty() => block.push(line),
            _ => {
                flush(&mut out, &mut block);
                out.push_str(line);
                out.push('\n');
            },
        }
    }

    flush(&mut out, &mut block);
    out
}

fn flush(out: &mut String, block: &mut Vec<&str>) {
    while block.last().is_some_and(|line| line.trim().is_empty()) {
        block.pop();
    }

    if block.is_empty() {
        return;
    }

    let list =
        block.iter().any(|line| line.trim_start().starts_with(['-', '*', '+']));

    if list {
        for line in block.drain(..) {
            out.push_str(line);
            out.push('\n');
        }
        return;
    }

    let dedent = block
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| leading_spaces(line))
        .min()
        .unwrap_or(0);

    out.push_str(FENCE);
    out.push('\n');
    for line in block.drain(..) {
        out.push_str(
            line.get(dedent..).unwrap_or_else(|| line.trim_start()).trim_end(),
        );
        out.push('\n');
    }
    out.push_str(FENCE);
    out.push('\n');
}

fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();

    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn language(open: &str) -> &'static str {
    let tag = open
        .trim()
        .trim_start_matches(['`', '~'])
        .split([' ', ','])
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    if tag.is_empty() {
        return "";
    }

    if let Some((_alias, mapped)) =
        ALIASES.iter().find(|(alias, _mapped)| *alias == tag)
    {
        return mapped;
    }

    KNOWN.iter().find(|known| **known == tag).copied().unwrap_or_default()
}

const KNOWN: &[&str] = &[
    "bash",
    "c",
    "cpp",
    "csharp",
    "css",
    "diff",
    "docker",
    "elixir",
    "go",
    "graphql",
    "html",
    "http",
    "ini",
    "java",
    "javascript",
    "json",
    "kotlin",
    "lua",
    "makefile",
    "markdown",
    "nginx",
    "nix",
    "perl",
    "php",
    "powershell",
    "python",
    "ruby",
    "rust",
    "scala",
    "sql",
    "swift",
    "toml",
    "typescript",
    "xml",
    "yaml",
];

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}
