use super::*;

const SIZE: f32 = 14.0;
const LH: f32 = 21.0;
const SMALL: f32 = 12.0;
const SMALL_LH: f32 = 14.0;
const INDENT: f32 = 12.5 * MM;

const SERIF: FontFamily = FontFamily {
    name: "GostSerif",
    fallback: "\"Times New Roman\", serif",
    files: [
        ("LiberationSerif-Regular.ttf", include_bytes!("../templates/fonts/LiberationSerif-Regular.ttf")),
        ("LiberationSerif-Italic.ttf", include_bytes!("../templates/fonts/LiberationSerif-Italic.ttf")),
        ("LiberationSerif-Bold.ttf", include_bytes!("../templates/fonts/LiberationSerif-Bold.ttf")),
        ("LiberationSerif-BoldItalic.ttf", include_bytes!("../templates/fonts/LiberationSerif-BoldItalic.ttf")),
    ],
};

const MONO: FontFamily = FontFamily {
    name: "GostMono",
    fallback: "\"Courier New\", monospace",
    files: [
        ("LiberationMono-Regular.ttf", include_bytes!("../templates/fonts/LiberationMono-Regular.ttf")),
        ("LiberationMono-Italic.ttf", include_bytes!("../templates/fonts/LiberationMono-Italic.ttf")),
        ("LiberationMono-Bold.ttf", include_bytes!("../templates/fonts/LiberationMono-Bold.ttf")),
        ("LiberationMono-BoldItalic.ttf", include_bytes!("../templates/fonts/LiberationMono-BoldItalic.ttf")),
    ],
};

pub(super) fn style() -> Style {
    let text = Role::text(SIZE, LH);
    let paragraph = Role { first: INDENT, align: Align::Justify, min_first: 2, min_last: 2, ..text };
    let title = Role { font: Font::BOLD, upper: true, align: Align::Center, space_after: LH, keep_with_next: true, ..text };
    let header = Role { first: INDENT, rest: INDENT, space_before: LH, space_after: LH / 2.0, keep_with_next: true, ..text };
    let small = Role::text(SMALL, SMALL_LH);
    let around = |space| Role { space_before: space, space_after: space, ..text };

    Style {
        name: DocStyle::Gost7_32,
        fonts: Fonts { body: SERIF, mono: MONO },
        page: Some(PageGeometry::A4),
        border: 0.75,
        sup_scale: 0.7,
        mono_bg: None,
        non_russian: Some(Font { italic: true, ..Font::MONO }),
        title_bar: TitleBar { bg: None, color: None, size: None },
        ext_link: ExtLink { color: None, under: true },
        chrome: Chrome {
            paper: Color::WHITE,
            ink: Color::BLACK,
            line: Color::BLACK,
            nav_bg: Color::WHITE,
            nav_color: Color::new(0x22, 0x22, 0x22),
            nav_current_bg: None,
            nav_current_color: Color::BLACK,
            nav_current_bar: Color::BLACK,
            nav_current_bold: true,
            nav_dir: Color::new(0x55, 0x55, 0x55),
            dir_case: DirCase::Upper,
            accent: Color::BLACK,
            nav_title: Some("Содержание"),
        },
        screen: Screen { column: 310.0 * MM, pad: [5.0 * MM, 15.0 * MM, 20.0 * MM, 30.0 * MM] },
        dark: None,
        nav_toggle: false,
        theme_toggle: false,
        page_toc: false,

        body: paragraph,
        comment: Role { font: Font::ITALIC, ..paragraph },
        comment_prefix: Some("Комментарий:"),
        comment_bar: None,
        doc_title: Role { break_before: true, ..title },
        headers: [
            Role { space_before: LH, ..title },
            Role { font: Font::BOLD, ..header },
            Role { font: Font::BOLD, ..header },
            header,
            header,
        ],
        numbering: Numbering { from: 2, to: 4, skip: 1, sep: ".", trailing: "" },
        list: Role { first: INDENT, align: Align::Justify, min_first: 2, min_last: 2, ..text },
        list_step: INDENT,
        list_hang: false,
        markers: Markers { ordered: "{n})\u{a0}", unordered: "–\u{a0}" },
        captions: Captions {
            table: Caption { named: "Таблица\u{a0}{n}\u{a0}—\u{a0}", anonymous: "Таблица\u{a0}{n}", role: text, caption_after: false, index_color: None },
            image: Caption { named: "Рисунок\u{a0}{n}\u{a0}— ", anonymous: "Рисунок\u{a0}{n}", role: Role { align: Align::Center, ..text }, caption_after: true, index_color: None },
            code: Caption { named: "Листинг\u{a0}{n}\u{a0}—\u{a0}", anonymous: "Листинг\u{a0}{n}", role: text, caption_after: false, index_color: None },
            table_cont: "Продолжение таблицы {n}",
            formula: "({n})",
        },

        figure: around(LH / 2.0),
        images: Images { gap: 6.0, px: 0.75, broken_h: 3.0 * LH },
        formula: around(LH),
        formula_cap: FormulaCap { role: paragraph, index_color: None, bar: None, head: None },
        code: Role { font: Font::MONO, min_first: 2, min_last: 2, ..Role { space_before: LH / 2.0, space_after: LH / 2.0, ..small } },
        code_head: None,
        code_frame: Frame { border: 0.75, pad_x: 6.0, pad_y: 4.0},
        code_border: true,
        code_numbers: false,
        code_radius: 0.0,
        highlight_code_syntax: false,
        table: Role { min_first: 1, min_last: 1, ..around(LH / 2.0) },
        cell: small,
        head_cell: Role { align: Align::Center, ..small },
        cell_frame: Frame { border: 0.75, pad_x: 4.0, pad_y: 2.0 },
        table_lines: TableLines::ALL,
        notes: Notes {
            role: Role { first: INDENT, align: Align::Justify, marg_after: 10.0, marg_before: 5.0,  ..small},
            sep: 10.0,
            rule: true,
            rule_w: 50.0 * MM,
            rule_top: 3.0,
            
        },
        toc: Some(Toc {
            title: "Содержание",
            max_level: 3,
            title_role: Role { break_before: false, ..title },
            entry: text,
        }),
    }
}
