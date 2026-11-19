# rush syntax

**[РУССКАЯ ВЕРСИЯ](SYNTAX_RU.md)**

> This is the English translation of the syntax reference. The Russian version, [SYNTAX_RU.md](SYNTAX_RU.md), is the original.

rush is designed for scientific documents and help pages, so in some places simplicity of syntax wins over generality. Such places are marked as a **trade-off**.

A note on Cyrillic: rush is meant to be typed on a Russian keyboard layout, so some tags and parameters use Cyrillic letters (`№`, `Л`/`Ц`/`П`, `ф`/`т`/`р`/`л`, `КРАС`, ...). Where a Latin alternative exists, it is listed next to the Cyrillic one. Generated captions (`Рисунок`, `Листинг`, ...) are in Russian.

## General rules

- The file is read as UTF-8. A UTF-8 BOM (`U+FEFF`) at the start of the file is ignored.
- Line endings: `\n` or `\r\n`. A lone `\r` inside a line is an ordinary character.
- **Whitespace** means only space and tab; non-breaking spaces and the like do not count. **Space** in tag descriptions means any whitespace character.
- An **empty line** is a line that contains only whitespace or nothing.
- An element is opened by a **tag** starting with `\`. Closing characters of inline elements (`:`, `;`, `)`, etc.) and table rows are written without `\`.
- Whitespace at the edges of a line does not prevent tag recognition: `   \+ Heading` is a heading. The exception is the content of code and formulas.
- A **block element** takes one or more whole lines. An **inline element** sits inside text.
- A **paired block** (comment, formula, code) is opened by a tag at the start of a line and lasts until the closing tag on its own line. Empty lines inside do not separate paragraphs.
- **Block isolation.** Inside an open block, tags of other blocks are not recognised and are treated as its content: `\// x` in a comment is text, `\"` in code is a line of code. Blocks do not overlap or nest: an open block lasts until its own closing tag.
- **Invalid paired block.** If there is no closing tag of the same kind on its own line before the end of the file, or a new opening tag of the same kind comes first, the opening line is dropped entirely, as if it were not there. The lines scanned during the search are parsed again as ordinary lines. Lines of other elements do not affect the search.
- **Stray closing tags.** `\//` or `\$` without an open block are dropped. `\№-` and `\№` outside a table become the text `№-` and `№`.
- A **triple block** (table) consists of the tags `\№ <NAME>`, `\№-` and `\№` on separate lines. An invalid table is not removed; it becomes ordinary text.

### Escaping

- The escape character `_` works on the special characters: `\` `_` `:` `;` `-` `=` `!` `%` `(` `)`.
- `_` before a special character is omitted, and the special character is output literally.
- Before any other character or at the end of a line, `_` is output as is.
- `__` is the character `_`.
- `_\` is the character `\`, after which no markup element starts.
- Examples: `\:10_:30:` is bold text `10:30`; `snake_case` is the text `snake_case`; `a__:` is the text `a_:`; `_\:` is the text `\:`.
- `_` at the start of a line cancels a block tag: `_\+ text` is a paragraph with the text `\+ text`.
- Closing characters (`:` `;` `-` `=` `!`) inside inline elements are escaped the same way.
- Escaped only in their own context:
  - `/` in table rows: `_/` does not separate cells. Outside a table `_/` is output as is.
  - `,` in an image line: `_,` does not separate parameters. Outside an image `_,` is output as is.
- Other tag characters need no escaping; they matter only after `\` or in a specific position:
  - `+` only in a heading (`\+`) and in merging table cells (`\+`, `\+=`);
  - `.` and `1` only as a list marker after `\`;
  - `*` only in footnotes and footnote references (`\**N`, `\*N`) and in element references (`\*р(...)`);
  - `$` only in the formula tag;
  - `№` only in table tags;
  - `,` in the link tag `\,`, and as a separator only in an image.
- Escaping does not apply in a table alignment row (`\№- Л / Ц`).
- Escaping does not apply inside formulas and code; in an image path only `_,` applies.

### Named elements

Formulas, code, tables and images have a name (caption) and are numbered in order, separately for each kind.

The name `0` makes an element **anonymous**: no caption and no number. Only a name exactly equal to `0` is anonymous; `\:0:` and `00` are ordinary names.

**Trade-off.** An element cannot be literally named `0`.

The caption has the form `<KIND> <NUMBER> - <NAME>`:

| Element | Kind (in the output) |
|---------|----------------------|
| Image   | Рисунок (Figure)     |
| Code    | Листинг (Listing)    |
| Table   | Таблица (Table)      |
| Formula | Формула (Formula)    |

For example: `Рисунок 2 - Схема узла`, `Листинг 1 - Пример функции`.

A name may contain inline elements. An **element reference** (`\*р(...)` etc.) inside a name is output as `??`. **Trade-off:** this rules out circular references.

If the name is empty after formatting is removed, the element is still numbered and captioned without a name (`Таблица 2 - `), but it cannot be referenced.

A space between the tag (`\$`, `\//`, `\№`) and the name, including `0`, is required; otherwise the line is read as ordinary text.

```
\$ 0
<LATEX FORMULA>
\$

\// 0
<CODE>
\//

\№ 0
...

\(<IMAGE PATH>, 0)
```


## Block elements

### Paragraph

Paragraphs are separated by one or more empty lines.

The lines of a paragraph are joined with a space. Word wrapping is done by the renderer; a manual hyphen (`wo-` + `rd`) stays in the text.

A block element inside a paragraph splits it into `paragraph`, `block element`, `paragraph`. Empty parts are dropped.

### Heading

```
\+ <TEXT>
\++ <TEXT>
...
\+++++ <TEXT>
```

- `\`, then `N` `+` characters and a mandatory space.
- `N` is the level from 1 to 5; more than five counts as five.
- Without text (`\+`, `\++   `) the line becomes paragraph text without `\`: `\+` leaves `+`.
- A heading that is empty only after formatting is removed (`\+ \:\:`) is output empty.
- The heading text is the rest of the line trimmed of whitespace; `+` inside it has no meaning: `\++ The C++ language`.
- A heading is a separate block: it interrupts a paragraph, and the next line starts a new one.

### List

```
\. <ITEM>
\ . <SECOND-LEVEL ITEM>
\  1 <THIRD-LEVEL ITEM>
\  1 <THIRD-LEVEL ITEM>
\. <ITEM>

\1 <ITEM>
\ 1 <SECOND-LEVEL ITEM>
```

- `\.` is an unordered list item.
- `\1` is an ordered list item. Numbering is automatic.
- Spaces between `\` and the marker set the nesting level: `1 + <number of spaces>`; a tab counts as 4 spaces.
- The marker must be followed by a space and non-empty text.
  - Otherwise (`\.text`, `\.`, `\1 `) the line becomes paragraph text without `\`.
- An item takes exactly one line.
- A list is a run of consecutive items. A line without a marker or an empty line ends it, and the next list starts numbering again.
- `\.` and `\1` items can be mixed. `\1` numbering interrupted by a `\.` item starts again.
- Skipped levels are displayed as the renderer decides.
- **Trade-off.** The ordered list marker is only `1`, and the starting number cannot be set: `\2 text` is the text `2 text`.

### Image

```
\(<IMAGE PATH>, <CAPTION>, <SCALE>)

\(<IMAGE PATH>, <CAPTION>)

\(<IMAGE PATH>)
```

- An image line starts with `\(` and ends with `)`.
- Parameters are separated by commas; whitespace around them is trimmed.
- A comma in the path or caption is escaped: `\(folder/a_,b.png, Diagram_, version 2, 120)` gives the path `folder/a,b.png` and the caption `Diagram, version 2`. **Trade-off:** a parameter cannot end with `_`.
- More than three parameters is ordinary text.
- `<IMAGE PATH>` is required, relative to the project root, with `/` separators. The extension is case-insensitive; `_` other than `_,` is taken as is.
- `<CAPTION>`: `0` or no caption at all (`\(path)`) makes the image anonymous.
- `<SCALE>` is optional: a percentage of the original size, an integer from `1` to `65535` (a leading `+` is allowed). `120` is 1.2 times larger.
- An invalid scale is ignored.
- A missing file or an empty path (`\()`) is output as a broken image.
- An image is a block element.

### Footnote

```
\**N <TEXT>
```

- Always starts on a new line.
- Footnotes on adjacent lines are combined into one block:

  ```
  \**1 First footnote
  \**2 Second footnote
  ```

  The block ends at an empty line, at any other line, and at an invalid footnote (`\**1text`), which becomes a paragraph. A dropped repeated definition does not interrupt the block.
- `N` is an integer from `0` to `65535`; leading zeros are allowed (`\**01` is footnote 1). The footnote text is the rest of the line after the space.
- Without a space or text (`\**1`, `\**1text`) it is ordinary text.
- `N` out of range (`\**70000 text`) is also text.
- `N = 0` is an anonymous footnote without references; there may be any number of them.
- For `N > 0`, repeated definitions with the same `N` are dropped.
- A footnote block is output where it is written, footnotes in order.
- Footnotes are numeric only.
- A footnote reference is the inline element `\*N`.

### Comment

```
\"
<CONTENT>
\"
```

- A paragraph highlighted with a background and colour; it may contain any inline elements.
- The tags stand alone on their lines: `\" text` is the paragraph text `" text`.
- A comment is a single paragraph; empty lines inside are lost. **Trade-off:** comments are meant for short notes.
- Block tags inside are not recognised: `\+ x` is the text `+ x`.
- There is no name, and the opening and closing tags are identical. The first `\"` line after the opening closes the comment; there are no nested comments.
- Without a closing `\"` the comment is invalid: with an odd number of `\"` lines the last one is dropped, and the text after it remains a paragraph.

### Formula

```
\$ <FORMULA NAME>
<LATEX FORMULA>
\$
```

- The content is not parsed and is passed on as LaTeX.
- **Trade-off.** The formula is the only element that requires the Latin layout. LaTeX is a temporary solution.
- `\$` followed by a space and text opens a formula; a lone `\$` closes it.
- **Trade-off.** A formula line cannot start with `\$` (a dollar sign in LaTeX): `\$ 100` opens a new formula, and a lone `\$` closes the current one.

### Code

```
\// <LISTING NAME>
fn foo(bar: i32) -> i32 {
    bar * 42
}
\//
```

- The content is not parsed. Indentation and empty lines are preserved; trailing whitespace is not.
- `\//` followed by a space and text always opens a new code block, even inside an unclosed one. Only a lone `\//` closes it.
- **Trade-off.** A line of code cannot start with `\//`.
- A **language label** is written right after `\//`: `\//py <LISTING NAME>`. The label (`py`, `rs`, `c++`) is not part of the name. `\//py` without a name is ordinary text. It is still closed by `\//`.
- The label is used for syntax highlighting: currently only Markdown output (```` ```py ````) and the `modern` style use it.

### Table

```
\№ <TABLE NAME>
/ Header 1 / Header 2 / Header 3 / Header 4 /
/ Subheader 1 / Subheader 2 / Subheader 3 / Subheader 4 /
\№- Л / Ц / П / Ц /
/ Cell / Cell / Cell / Cell /
/ Cell merged horizontally across two others / \+ / \+ / Cell /
/ Cell merged with the one below / Cell / Cell / Cell /
/ \= / Cell / Cell / Cell /
/ Cell / Cell merged from 4 / \+ / Cell /
/ Cell / \= / \+= / Cell /
\№
```

A table contains exactly three tags, each at the start of a line:

1. **`\№ <TABLE NAME>`** starts the table. `\№ -` (with a space) starts a table named `-`.
2. **`\№-`** is the separator: at least one header row above it, the content below. After `\№-` and a space you may specify column alignment: `Л` is left, `Ц` is centre, `П` is right (centre by default). `L` may be used instead of `Л`, `C` instead of `Ц`, and `R` instead of `П`.
3. **`\№`** (alone on a line) ends the table. There may be no content rows.
4. **`\#`** may be used instead of **`\№`** to type tables on the English layout, since `№` is the only operator that cannot be typed on an English QWERTY layout.

Only table rows are allowed between the tags.

**Invalid table.** A table is invalid if:

- after the start there is a line that is neither a table row nor the next tag (an empty line, text, `/Cell /`, a new `\№ <NAME>`);
- there is no header row between the start and the separator;
- the tags are out of order, or the file ends before the end of the table.

Then the start line becomes the paragraph text `№ <NAME>`, and the following lines are parsed again. A new `\№ <NAME>` opens a new table:

```
\№ Incomplete table
/ Header / Header /
\№ New table
/ Header / Header /
\№-
/ cell / cell /
\№
```

The first two lines are a paragraph; the rest is the table "New table".

The number of columns equals the largest number of cells in the header rows; merging and empty cells count too.

- Short rows are padded on the right with empty cells.
- Long rows are truncated on the right: only the header sets the number of columns.

Alignment:

- Alignments are separated by ` / `; the outer `/` are optional: `\№- Л / Ц` = `\№- / Л / Ц /`.
- The `i`-th alignment applies to the `i`-th column.
- Case-insensitive.
- Columns without an alignment or with an unrecognised value are centred.
- Extra alignments are ignored.
- A merged cell is aligned according to the column of its top-left cell.

Column widths:

- A column weight can be written next to the alignment letter, without a space: `Ц100`, `50Л`, `200` (centred).
- A column takes a share of the width equal to its weight divided by the sum of weights.
- Columns without a weight get the average of the specified weights; extra values are ignored.
- An unrecognised value (`0Ц`, `Ц 10`, `10Ц20`) is centred, without a weight.
- Without weights, widths are fitted to the content.
- Example: `\№- / Ц100 / 200 / Ц / 50Ц /`: the third column gets the average ~116.7, giving shares of ~21 %, 43 %, 25 % and 11 %.
- In Markdown, weights are ignored with a warning.

### Table rows

- A table row starts with `/ ` and ends with ` /`. `/Cell /`, `/ Cell/` and `/ Cell` are not table rows.
- Cells are separated by ` / ` (spaces on both sides).
- Any other `/` is text: `/ km/h / 5 /` is two cells, `km/h` and `5`.
- ` / ` inside a cell is escaped: `/ a _/ b /` is the cell `a / b`.
- Whitespace at the edges of a cell is trimmed.
- An empty cell is whitespace between separators: `/ / /` is two empty cells, `/ /` is one.
- A cell is parsed as a separate line: in `/ \:a / b: /` only `a` is bold.

### Merging cells

- `\+` merges the cell with the one on the left.
- `\=` merges the cell with the one above.
- `\+=` (or `\=+`) merges the cell with the ones above and on the left.
- A cell is a merging cell only if, trimmed of whitespace, it equals `\+`, `\=`, `\+=` or `\=+`. Otherwise `\=` starts underlined text.
- A literal `\+` or `\=` is escaped: `/ _\+ /`.
- The text of a merged cell is written into its top-left cell.
- A content cell cannot be merged with a header cell.
- An impossible merge gives an empty cell:
  - `\+` in the first column;
  - `\=` in the first header row or the first content row;
  - `\+=`/`\=+` if at least one side is impossible.
- Non-rectangular merges are allowed; the renderer decides how to display them.


## Inline elements

- Inline elements nest inside each other, except coloured text and links inside an element of their own kind.
- A closing character closes the most recently opened element of its kind. Unclosed elements inside it are invalid: the tag is omitted, the text remains.
  - `\:a \:b: c:` is bold text `a` containing bold `b`, then ` c`.
  - `\:a \;b: c;` is bold text `a b` (the invalid italic start `\;` is omitted), then plain text ` c;`.
- The end of a line closes all inline elements.
  - `\:unclosed` (end of line) is bold text `unclosed`.
  - `\:a \;b c` (end of line) is bold text `a b c` containing italic `b c`.
  - An unclosed element is valid; one cut off by the closing of an outer element is not: `\:a \;b c` versus `\:a \;b: c`.
- Inside an element, the first closing character closes it: in `\:Time: 10:30` only `Time` is bold. Escaped: `\:Time_: 10_:30:`.
- `\` before a character that is not a tag is omitted: `\Abc` is the text `Abc`, `\\:x:` is bold `x`.
- `\` at the end of a line is also omitted; a literal `\` is `_\`.
- Surrounding formatting applies to footnote and element references.

### Text in parentheses

Links, coloured text and element references are written as a tag, a parameter and text in parentheses.

```
\,<TARGET>(<TEXT>)
\%<COLOUR>(<TEXT>)
\*<KIND>(<NAME>)
```

- The parameter ends at the first `(`; escaping does not apply in it. A target containing parentheses is written in quotes.
- If there is no `(` before the end of the line, the `\,` or `\%` tag is dropped together with the rest of the line: `before \%КРАС after` is the text `before `.
- For an element reference the parameter is a single letter immediately followed by `(`. Otherwise only `\*` is omitted: `\*р abc` is the text `р abc`.
- Parentheses inside are balanced, so paired ones need no escaping: `\*р(Diagram (version 2))` is the name `Diagram (version 2)`.
- An unpaired parenthesis must be escaped: `\*р(Step 1_) setup)` is the name `Step 1) setup`. Paired ones need not be: `\,google.com(google(search))`.
- Parentheses outside an element do not affect parsing: `a) \%КРАС(b)`.
- An element with empty text (`\%КРАС()`) is dropped.
- Coloured text and links do not nest inside an element of their own kind: the inner tag is omitted, and its text remains in the outer element.
  - `\%КРАС(a \%СИН(b) c)` is red text `a b c`.
  - `\,a.ru(x \,b.ru(y) z)` is one link to `a.ru` with the text `x y z`.
  - This applies only if the outer element is valid. An unrecognised `<COLOUR>` does not make the element invalid.
  - Different colours are written one after another, not nested: `\:\%КРАС(Bold) \%СИН(blue) \%КРАС(red):`.

### Text formatting

| Syntax                | Result                    |
|-----------------------|---------------------------|
| `\:<TEXT>:`           | bold text                 |
| `\;<TEXT>;`           | italic text               |
| `\-<TEXT>-`           | strikethrough text        |
| `\=<TEXT>=`           | underlined text           |
| `\!<TEXT>!`           | monospace text            |
| `\%<COLOUR>(<TEXT>)`  | text in colour `<COLOUR>` |

For example: `\%КРАС(warning)` or `\%RED(warning)`.

`<COLOUR>` is not trimmed (`\%КРАС (x)` is not recognised) and is given in one of these ways:

- HEX: `#` and 6 hexadecimal digits: `#1b2b20`.
- HEX in Russian notation: `Ц` and 6 digits, where `a`=`а` `b`=`б` `c`=`в` `d`=`г` `e`=`д` `f`=`е`: `ц1б2б20`. Case-insensitive; Latin `a`–`f` are not recognised.
- RGB: 9 digits, three per channel, each no greater than `255`: `255103020`.
- An upper-case constant; in Russian names `Ё` may be replaced with `Е` (`ЖЕЛ`):

| Constant              | Colour     |
|-----------------------|------------|
| `КРАС` or `RED`       | red        |
| `ОРАН` or `ORANGE`    | orange     |
| `ЖЁЛ` or `YELLOW`     | yellow     |
| `ЗЕЛ` or `GREEN`      | green      |
| `ГОЛ` or `LIGHTBLUE`  | light blue |
| `СИН` or `BLUE`       | blue       |
| `ФИОЛ` or `PURPLE`    | purple     |
| `РОЗ` or `PINK`       | pink       |
| `КОР` or `BROWN`      | brown      |
| `ЛИМ` or `LEMON`      | lemon      |
| `САЛ` or `SALAT`      | chartreuse |
| `БИР` or `CYAN`       | cyan       |
| `ЛАЗ` or `AZURE`      | lazuli     |
| `ФУК` or `MAGENTA`    | fuchsia    |
| `БЕЛ` or `WHITE`      | white      |
| `ЧЁРН` or `BLACK`     | black      |

An unrecognised `<COLOUR>` gives uncoloured text, but the element remains valid: `\%XYZ(a \%СИН(b) c)` is `a b c` without colour.

### Link

```
\,<TARGET>(<TEXT>)
```

- `<TARGET>` is required; whitespace at its edges is trimmed: `\, a.ru (x)` is a link to `a.ru`.
- A quoted target `\,"<TARGET>"(<TEXT>)` is taken as is, without escaping: `\,"a.ru/page(low)"(text)`.
  - `(` must come right after the closing quote; otherwise the tag is dropped with the rest of the line.
- Internal and external links are written the same way.
- If `<TARGET>` is one of the compiled `.rush` documents, the link is internal; otherwise it is external (a URL). An external link that matches a document name is written with a protocol: `\,"https://help"(text)`.
- An internal target is relative to the project root, with `/` separators: `\,section/help(text)`. The name is case-sensitive, the extension is not. There are no anchors.
- The `.rush` extension may be omitted: `\,help(text)` and `\,help.rush(text)` are equivalent.
- Links do not nest: the outer one wins.
- Adjacent links with the same `<TARGET>` and no text between them are merged, so a long link can be split across lines:

  ```
  \,help(a very long text
  \,help(of a link to the help page)
  ```

  This is a single link. Links separated by text or a space (`\,a.ru(x) \,a.ru(y)`) stay separate.

### Footnote reference

```
\*N
```

- A reference to footnote `\**N` of this document; `N` is from `0` to `65535`, leading zeros are allowed. `\*0` is output without a jump target.
- A number greater than `65535` is the text `*70000`.
- A footnote may have any number of references.
- A reference to a non-existent footnote is output without a jump target.
- The reference ends with the number, and the space after it is removed (the renderer inserts it): `Barents\*12sea` and `war\*12 of 1912` both reference footnote 12.

### Element reference

```
\*ф(<NAME>)
\*т(<NAME>)
\*р(<NAME>)
\*л(<NAME>)
```

- Expands to the number of a named element.
- The lower-case letter after `\*` sets the kind: `ф` is a formula, `т` a table, `р` an image, `л` a code listing.
- Example: `\*р(Example)` expands to `2` if `Example` is the name of image 2.
- Elements further down the text can be referenced too.
- Inside an element name, a reference is output as `??`.
- When names are compared:
  - formatting is ignored: `\%КРАС(Node) diagram` -> `\*р(Node diagram)`;
  - a link in the name leaves its text, and footnote and element references are ignored: `\,site.ru(Node) diagram\*1` -> `\*р(Node diagram)`;
  - escapes are resolved: `\:10_:30:` -> `\*р(10:30)`;
  - case is ignored;
  - `ё` and `е` are treated as the same letter;
  - consecutive whitespace counts as one space, and whitespace at the edges is ignored.
- If several names match, the first element is taken.
- A reference that is not found is output as `??`.

Alternative letters can be used instead of the standard ones:

| Target     | Standard | Alternatives  |
|------------|----------|---------------|
| Formula    | `ф`      | `f`           |
| Table      | `т`      | `t`           |
| Code block | `л`      | `к`; `l`; `c` |
| Image      | `р`      | `и`; `d`; `i` |

## Element nesting

| Element | May contain |
|---------|-------------|
| Paragraph, list item, table cell, heading, footnote text, comment | inline elements |
| Element name | inline elements, except element references (`??`) |
| Inline element | inline elements |
| Formula, code | nothing (the content is not parsed) |
