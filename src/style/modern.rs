use super::*;

const SIZE: f32 = 12.0;
const LH: f32 = 18.0;
const SMALL: f32 = 11.0;
const SMALL_LH: f32 = 13.0;
const INDENT: f32 = 10.0 * MM;

const SANS_SERIF: FontFamily = FontFamily {
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


pub(super) fn style() -> Style {
    let text = Role::text(SIZE, LH);
    let paragraph = Role {first: INDENT, align: Align::Left, min_first: 2, min_last: 2, ..text};
    let title = Role {font: Font::BOLD, space_after: LH, align: Align::Center, keep_with_next: true, ..text };
    let header = Role {font: Font::BOLD, space_before: LH, space_after: LH / 2.0, keep_with_next: true,  ..text};
    let h1 = Role {size: SIZE * 2.0, ..header};
    let small = Role::text(SMALL, SMALL_LH);
    let around = |space| Role {space_before: space, space_after: space, ..text};
    let caption = Role {text_color: Some(Color::new(140, 140, 140)), font: Font::ITALIC, align: Align::Right, space_after: 10.0, ..small};
    
    Style {
         name: DocStyle::Modern, 
         fonts: Fonts { body: SANS_SERIF, mono: MONO },
         page: Some(PageGeometry { number_align: Align::Right, ..PageGeometry::A4 }),
         border: 0.5, 
         sup_scale: 0.7, 
         mono_bg: Some(Color::new(0xee, 0xee, 0xee)),
         non_russian: None,
         ext_link: ExtLink { color: Some(Color::new(0x1a, 0x5f, 0xd6)), under: true },
         title_bar: TitleBar { bg: Some(Color::new(0x49, 0x4f, 0x60)), color: Some(Color::new(255, 255, 255)) },
         
         body: paragraph, 
         comment: Role {font: Font::ITALIC, first: INDENT / 2.0, bg_color: Some(Color::new(233,233, 233)), text_color: Some(Color::new(120, 120, 120)), squeeze: 20.0, ..paragraph}, 
         comment_prefix: Some(">"), 
         
         doc_title: Role {break_before: true, ..h1},

         headers: [
            h1,
            Role{size: SIZE * 1.75, ..header},
            Role{size: SIZE * 1.5, ..header},
            Role{size: SIZE * 1.25, ..header},
            header,
         ], 
         
         numbering: Numbering { from: 1, to: 5, skip: 0, sep: ".", trailing: "." }, 
         list: Role {first: INDENT * 1.5, min_first: 4, min_last: 4, marg_before: LH / 2.0, marg_after: LH / 2.0, ..text},
         list_step: INDENT / 2.0,
         list_hang: true,
         markers: Markers { ordered: "{n}.\u{a0}", unordered: "•\u{a0}" },
         captions: Captions { 
            table: Caption { named: "Табл.\u{a0}{n}:\u{a0}\u{a0}", anonymous: "", role: caption, caption_after: false },
            image: Caption { named: "Рис.\u{a0}{n}:\u{a0}\u{a0}", anonymous: "", role: caption, caption_after: false },
            code: Caption { named: "{n}\u{a0}|\u{a0}", anonymous: "", role: Role {font: Font::MONO, align: Align::Left, text_color: Some(Color::new(220, 220, 180)), ..caption}, caption_after: false },
            table_cont: "Табл. {n}",
            formula: "({n})", 
        },
        figure: around(LH / 2.0),
        images: Images { gap: 6.0, px: 0.75, broken_h: 3.0 * LH },
        formula: around(LH),
        code: Role { font: Font::MONO, min_first: 2, min_last: 2, marg_before: 20.0, marg_after: 20.0, squeeze: 16.0, bg_color: Some(Color::new(0, 0, 0)), text_color: Some(Color::new(220, 220, 220)), ..Role { space_before: LH / 2.0, space_after: LH / 2.0, ..small } },
        code_frame: Frame { border: 0.75, pad_x: 6.0, pad_y: 4.0 },
        code_border: false,
        code_numbers: true,
        table: Role { min_first: 1, min_last: 1, ..around(LH / 2.0) },
        cell: small,
        head_cell: Role { align: Align::Center, font: Font::BOLD, bg_color: Some(Color::new(222, 184, 135)), ..small },
        cell_frame: Frame { border: 0.75, pad_x: 4.0, pad_y: 2.0 },
        table_lines: TableLines::ALL,
        notes: Notes {
            role: Role { align: Align::Right, bg_color: Some(Color::new(233, 233, 233)), marg_before: 2.0 * LH, marg_after: 2.0 * LH, squeeze: 8.0, ..small },
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