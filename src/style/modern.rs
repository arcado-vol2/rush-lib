use super::*;

const SIZE: f32 = 12.0;
const LH: f32 = 18.0;
const SMALL: f32 = 11.0;
const SMALL_LH: f32 = 13.0;
const INDENT: f32 = 10.0 * MM;

const SERIF: FontFamily = FontFamily {
    name: "ModernRoboto",
    fallback: "\"Arial\", sans-serif",
    files: [
        ("Roboto-Regular.ttf", include_bytes!("../templates/fonts/Roboto-Regular.ttf")),
        ("Roboto-Italic.ttf", include_bytes!("../templates/fonts/Roboto-Italic.ttf")),
        ("Roboto-Bold.ttf", include_bytes!("../templates/fonts/Roboto-Bold.ttf")),
        ("Roboto-BoldItalic.ttf", include_bytes!("../templates/fonts/Roboto-BoldItalic.ttf")),
    ],
};

const MONO: FontFamily = FontFamily {
    name: "ModernJetBrainsMono",
    fallback: "\"Consolas\", \"Courier New\", monospace",
    files: [
        ("JetBrainsMono-Regular.ttf", include_bytes!("../templates/fonts/JetBrainsMono-Regular.ttf")),
        ("JetBrainsMono-Italic.ttf", include_bytes!("../templates/fonts/JetBrainsMono-Italic.ttf")),
        ("JetBrainsMono-Bold.ttf", include_bytes!("../templates/fonts/JetBrainsMono-Bold.ttf")),
        ("JetBrainsMono-BoldItalic.ttf", include_bytes!("../templates/fonts/JetBrainsMono-BoldItalic.ttf")),
    ],
};

const ACCENT: Color = Color { r: 0x9a, g: 0x2e, b: 0x1c };
const TERRACOTTA: Color = Color { r: 0xb3, g: 0x5a, b: 0x1f };

const CODE_HEAD_BG: Color = Color { r: 0xea, g: 0xe5, b: 0xdd };
const CODE_HEAD_INK: Color = Color { r: 0x4a, g: 0x40, b: 0x38 };
const CODE_HEAD_RULE: Color = Color { r: 0xd5, g: 0xce, b: 0xc2 };

pub(super) fn style() -> Style {
    let text = Role::text(SIZE, LH);
    let paragraph = Role {first: INDENT, align: Align::Justify, min_first: 2, min_last: 2, ..text};
    let title = Role {font: Font::BOLD, space_after: LH, align: Align::Center, keep_with_next: true, ..text };
    let header = Role {font: Font::BOLD, space_before: LH, space_after: LH / 2.0, keep_with_next: true,  ..text};
    let sized = |k: f32| Role {size: SIZE * k, lh: (SIZE * k * 1.25).max(LH), ..header};
    let h1 = sized(1.5);
    let small = Role::text(SMALL, SMALL_LH);
    let around = |space| Role {space_before: space, space_after: space, ..text};
    let caption = Role {text_color: Some(Color::new(110, 100, 90)), font: Font::ITALIC, align: Align::Right, space_after: 10.0, ..small};

    Style {
         name: DocStyle::Modern,
         fonts: Fonts { body: SERIF, mono: MONO },
         page: Some(PageGeometry { number_align: Align::Right, ..PageGeometry::A4 }),
         border: 0.5,
         sup_scale: 0.7,
         mono_bg: Some(Color::new(0xf1, 0xee, 0xe8)),
         non_russian: None,
         ext_link: ExtLink { color: Some(ACCENT), under: true },
         title_bar: TitleBar { bg: Some(Color::new(0x7a, 0x26, 0x18)), color: Some(Color::new(0xfd, 0xf6, 0xec)), size: None },
         chrome: Chrome {
            paper: Color::WHITE,
            ink: Color::new(0x1e, 0x1b, 0x18),
            line: Color::new(0x4a, 0x40, 0x38),
            nav_bg: Color::new(0xfa, 0xf8, 0xf3),
            nav_color: Color::new(0x3d, 0x37, 0x32),
            nav_current_bg: Some(Color::new(0xf3, 0xe6, 0xd8)),
            nav_current_color: Color::new(0x5c, 0x1d, 0x12),
            nav_current_bar: ACCENT,
            nav_current_bold: false,
            nav_dir: TERRACOTTA,
            dir_case: DirCase::SmallCaps,
            accent: ACCENT,
            nav_title: Some("Содержание"),
         },
         screen: Screen { column: 210.0 * MM, pad: [5.0 * MM, 15.0 * MM, 20.0 * MM, 20.0 * MM] },
         dark: None,
         nav_toggle: true,
         theme_toggle: false,
         page_toc: true,

         body: paragraph,
         comment: Role {font: Font::ITALIC, first: INDENT / 2.0, bg_color: Some(Color::new(0xf4, 0xef, 0xe8)), text_color: Some(Color::new(110, 100, 90)), squeeze: 20.0, ..paragraph},
         comment_prefix: Some(">"),
         comment_bar: None,

         doc_title: Role {break_before: true, ..h1},

         headers: [
            h1,
            sized(1.3),
            sized(1.15),
            header,
            header,
         ],

         numbering: Numbering { from: 1, to: 5, skip: 0, sep: ".", trailing: "." },
         list: Role {first: INDENT * 1.5, min_first: 4, min_last: 4, marg_before: LH / 2.0, marg_after: LH / 2.0, ..text},
         list_step: INDENT / 2.0,
         list_hang: true,
         markers: Markers { ordered: "{n}.\u{a0}", unordered: "•\u{a0}" },
         captions: Captions {
            table: Caption { named: "Табл.\u{a0}{n}:\u{a0}\u{a0}", anonymous: "", role: caption, caption_after: false, index_color: Some(TERRACOTTA) },
            image: Caption { named: "Рис.\u{a0}{n}:\u{a0}\u{a0}", anonymous: "", role: caption, caption_after: false, index_color: Some(TERRACOTTA) },
            code: Caption { named: "{n}\u{a0}|\u{a0}", anonymous: "", role: Role {font: Font { bold: true, ..Font::MONO }, align: Align::Left, bg_color: Some(CODE_HEAD_BG), text_color: Some(CODE_HEAD_INK), ..caption}, caption_after: false, index_color: Some(TERRACOTTA) },
            table_cont: "Табл. {n}",
            formula: "({n})",
        },
        figure: around(LH / 2.0),
        images: Images { gap: 6.0, px: 0.75, broken_h: 3.0 * LH },
        formula: around(LH),
        code: Role { font: Font::MONO, min_first: 2, min_last: 2, marg_before: 20.0, marg_after: 20.0, squeeze: 16.0, bg_color: Some(Color::new(0xf3, 0xf3, 0xf3)), text_color: Some(Color::new(0x22, 0x22, 0x22)), ..Role { space_before: LH / 2.0, space_after: LH / 2.0, ..small } },
        code_head: Some(CodeHead { rule: CODE_HEAD_RULE, rule_w: 0.75, pad: [4.0, 5.0, 4.0, 16.0], gap: 6.0 }),
        code_frame: Frame { border: 0.75, pad_x: 6.0, pad_y: 4.0 },
        code_border: false,
        code_numbers: true,
        code_radius: 0.0,
        highlight_code_syntax: true,
        table: Role { min_first: 1, min_last: 1, ..around(LH / 2.0) },
        cell: small,
        head_cell: Role { align: Align::Center, font: Font::BOLD, bg_color: Some(Color::new(222, 184, 135)), ..small },
        cell_frame: Frame { border: 0.75, pad_x: 4.0, pad_y: 2.0 },
        table_lines: TableLines::ALL,
        notes: Notes {
            role: Role { align: Align::Right, bg_color: Some(Color::new(0xf4, 0xef, 0xe8)), marg_before: 2.0 * LH, marg_after: 2.0 * LH, squeeze: 8.0, ..small },
            sep: 10.0,
            rule: false,
            rule_w: 150.0 * MM,
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
