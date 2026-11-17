use super::*;

const SIZE: f32 = 13.5;
const LH: f32 = 23.0;
const SMALL: f32 = 11.0;
const SMALL_LH: f32 = 17.0;
const CODE: f32 = 11.0;
const CODE_LH: f32 = 17.0;
const STEP: f32 = 20.0;

const SANS_SERIF: FontFamily = FontFamily {
    name: "BlogRoboto",
    fallback: "\"Open Sans\", \"Segoe UI\", \"Helvetica Neue\", Arial, sans-serif",
    files: [
        ("Roboto-Regular.ttf", include_bytes!("../templates/fonts/Roboto-Regular.ttf")),
        ("Roboto-Italic.ttf", include_bytes!("../templates/fonts/Roboto-Italic.ttf")),
        ("Roboto-Bold.ttf", include_bytes!("../templates/fonts/Roboto-Bold.ttf")),
        ("Roboto-BoldItalic.ttf", include_bytes!("../templates/fonts/Roboto-BoldItalic.ttf")),
    ],
};

const MONO: FontFamily = FontFamily {
    name: "BlogJetBrainsMono",
    fallback: "\"Consolas\", \"Source Code Pro\", monospace",
    files: [
        ("JetBrainsMono-Regular.ttf", include_bytes!("../templates/fonts/JetBrainsMono-Regular.ttf")),
        ("JetBrainsMono-Italic.ttf", include_bytes!("../templates/fonts/JetBrainsMono-Italic.ttf")),
        ("JetBrainsMono-Bold.ttf", include_bytes!("../templates/fonts/JetBrainsMono-Bold.ttf")),
        ("JetBrainsMono-BoldItalic.ttf", include_bytes!("../templates/fonts/JetBrainsMono-BoldItalic.ttf")),
    ],
};

struct Palette {
    chrome: Chrome,
    bar_bg: Color,
    bar_color: Color,
    link: Color,
    muted: Color,
    code_bg: Color,
    code_color: Color,
    code_cap: Color,
    code_rule: Color,
    mono_bg: Color,
    head_bg: Color,
    comment: Color,
    comment_bar: Color,
}

const LIGHT: Palette = Palette {
    chrome: Chrome {
        paper: Color { r: 0xff, g: 0xff, b: 0xff },
        ink: Color { r: 0x26, g: 0x26, b: 0x26 },
        line: Color { r: 0xe0, g: 0xe0, b: 0xe0 },
        nav_bg: Color { r: 0xfa, g: 0xfa, b: 0xfa },
        nav_color: Color { r: 0x36, g: 0x41, b: 0x49 },
        nav_current_bg: None,
        nav_current_color: Color { r: 0x10, g: 0x80, b: 0xd0 },
        nav_current_bar: Color { r: 0x10, g: 0x80, b: 0xd0 },
        nav_current_bold: false,
        nav_dir: Color { r: 0x8c, g: 0x8c, b: 0x8c },
        dir_case: DirCase::Plain,
        accent: Color { r: 0x10, g: 0x80, b: 0xd0 },
        nav_title: None,
    },
    bar_bg: Color { r: 0xff, g: 0xff, b: 0xff },
    bar_color: Color { r: 0x33, g: 0x33, b: 0x33 },
    link: Color { r: 0x41, g: 0x83, b: 0xc4 },
    muted: Color { r: 0x8c, g: 0x8c, b: 0x8c },
    code_bg: Color { r: 0xf6, g: 0xf7, b: 0xf6 },
    code_color: Color { r: 0x33, g: 0x33, b: 0x33 },
    code_cap: Color { r: 0xed, g: 0xf0, b: 0xf2 },
    code_rule: Color { r: 0xdd, g: 0xe2, b: 0xe6 },
    mono_bg: Color { r: 0xf0, g: 0xf1, b: 0xf2 },
    head_bg: Color { r: 0xf6, g: 0xf7, b: 0xf8 },
    comment: Color { r: 0x55, g: 0x55, b: 0x55 },
    comment_bar: Color { r: 0x41, g: 0x83, b: 0xc4 },
};

const DARK: Palette = Palette {
    chrome: Chrome {
        paper: Color { r: 0x14, g: 0x16, b: 0x17 },
        ink: Color { r: 0xc5, g: 0xcb, b: 0xd1 },
        line: Color { r: 0x3b, g: 0x40, b: 0x45 },
        nav_bg: Color { r: 0x29, g: 0x2c, b: 0x2f },
        nav_color: Color { r: 0xa1, g: 0xad, b: 0xb8 },
        nav_current_bg: None,
        nav_current_color: Color { r: 0x3b, g: 0xa0, b: 0xe0 },
        nav_current_bar: Color { r: 0x3b, g: 0xa0, b: 0xe0 },
        nav_current_bold: false,
        nav_dir: Color { r: 0x77, g: 0x80, b: 0x88 },
        dir_case: DirCase::Plain,
        accent: Color { r: 0x3b, g: 0xa0, b: 0xe0 },
        nav_title: None,
    },
    bar_bg: Color { r: 0x14, g: 0x16, b: 0x17 },
    bar_color: Color { r: 0xc5, g: 0xcb, b: 0xd1 },
    link: Color { r: 0x2b, g: 0x9b, b: 0xd8 },
    muted: Color { r: 0x7b, g: 0x84, b: 0x8c },
    code_bg: Color { r: 0x1d, g: 0x1f, b: 0x21 },
    code_color: Color { r: 0xc5, g: 0xc8, b: 0xc6 },
    code_cap: Color { r: 0x23, g: 0x27, b: 0x2a },
    code_rule: Color { r: 0x3a, g: 0x40, b: 0x46 },
    mono_bg: Color { r: 0x1d, g: 0x1f, b: 0x21 },
    head_bg: Color { r: 0x1f, g: 0x22, b: 0x25 },
    comment: Color { r: 0x9a, g: 0xa3, b: 0xab },
    comment_bar: Color { r: 0x2b, g: 0x79, b: 0xa2 },
};

pub(super) fn style() -> Style {
    styled(&LIGHT)
}

fn dark() -> Style {
    styled(&DARK)
}

fn styled(p: &Palette) -> Style {
    let text = Role::text(SIZE, LH);
    let paragraph = Role { space_after: SIZE, min_first: 2, min_last: 2, ..text };
    let header = Role { font: Font::BOLD, keep_with_next: true, ..text };
    let sized = |k: f32| Role { size: SIZE * k, lh: (SIZE * k * 1.3).round(), space_before: SIZE * 1.6, space_after: SIZE * 0.6, ..header };
    let h1 = sized(2.0);
    let small = Role::text(SMALL, SMALL_LH);
    let around = |space| Role { space_before: space, space_after: space, ..text };
    let caption = |align| Role { align, space_after: 6.0, ..small };

    Style {
        name: DocStyle::Blog,
        fonts: Fonts { body: SANS_SERIF, mono: MONO },
        page: Some(PageGeometry::A4),
        border: 0.75,
        sup_scale: 0.7,
        mono_bg: Some(p.mono_bg),
        non_russian: None,
        title_bar: TitleBar { bg: Some(p.bar_bg), color: Some(p.bar_color), size: Some(15.0) },
        ext_link: ExtLink { color: Some(p.link), under: false },
        chrome: p.chrome,
        screen: Screen { column: 600.0 + 2.0 * 30.0, pad: [12.0, 30.0, 60.0, 30.0] },
        dark: Some(dark),
        nav_toggle: true,
        theme_toggle: true,
        page_toc: true,

        body: paragraph,
        comment: Role { text_color: Some(p.comment), squeeze: 16.0, marg_before: 4.0, marg_after: 4.0, ..paragraph },
        comment_prefix: None,
        comment_bar: Some(p.comment_bar),
        doc_title: Role { break_before: true, space_before: 0.0, ..h1 },
        headers: [h1, sized(1.6), sized(1.35), sized(1.15), sized(1.0)],
        // заголовки без номеров
        numbering: Numbering { from: 6, to: 0, skip: 0, sep: ".", trailing: "" },
        list: Role { first: STEP, min_first: 2, min_last: 2, marg_before: LH / 4.0, marg_after: SIZE, ..text },
        list_step: STEP,
        list_hang: true,
        markers: Markers { ordered: "{n}.\u{a0}", unordered: "•\u{a0}" },
        captions: Captions {
            table: Caption { named: "{n}\u{a0}\u{a0}", anonymous: "", role: caption(Align::Left), caption_after: false, index_color: Some(p.muted) },
            image: Caption { named: "{n}\u{a0}\u{a0}", anonymous: "", role: Role { space_after: 0.0, space_before: 6.0, ..caption(Align::Center) }, caption_after: true, index_color: Some(p.muted) },
            code: Caption { named: "{n}\u{a0}\u{a0}", anonymous: "", role: Role { font: Font::BOLD, bg_color: Some(p.code_cap), text_color: Some(p.code_color), ..caption(Align::Left) }, caption_after: false, index_color: Some(p.muted) },
            table_cont: "{n}",
            formula: "({n})",
        },

        figure: around(LH / 2.0),
        images: Images { gap: 6.0, px: 0.75, broken_h: 3.0 * LH },
        formula: around(LH / 2.0),
        code: Role {
            font: Font::MONO, size: CODE, lh: CODE_LH, min_first: 2, min_last: 2,
            marg_before: LH / 2.0, marg_after: LH, squeeze: 14.0,
            bg_color: Some(p.code_bg), text_color: Some(p.code_color),
            ..Role::text(CODE, CODE_LH)
        },
        code_frame: Frame { border: 0.75, pad_x: 6.0, pad_y: 4.0 },
        code_head: Some(CodeHead { rule: p.code_rule, rule_w: 0.75, pad: [4.0, 5.0, 4.0, 16.0], gap: 6.0 }),
        code_border: false,
        code_numbers: true,
        code_radius: 4.0,
        highlight_code_syntax: true,
        table: Role { min_first: 1, min_last: 1, marg_before: LH / 2.0, marg_after: LH, ..text },
        cell: Role::text(12.0, 18.0),
        head_cell: Role { font: Font::BOLD, bg_color: Some(p.head_bg), ..Role::text(12.0, 18.0) },
        cell_frame: Frame { border: 0.75, pad_x: 9.0, pad_y: 5.0 },
        table_lines: TableLines {
            frame: false,
            head: Grid { rows: true, cols: false },
            body: Grid { rows: true, cols: false },
        },
        notes: Notes {
            role: Role { marg_before: LH, marg_after: LH / 2.0, ..small },
            sep: 10.0,
            rule: true,
            rule_w: 120.0,
            rule_top: 3.0,
        },
        toc: Some(Toc {
            title: "Содержание",
            max_level: 3,
            title_role: Role { break_before: false, space_after: SIZE, ..h1 },
            entry: text,
        }),
    }
}
