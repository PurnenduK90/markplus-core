---
title: Inline Extensions Sample
author:
  name: Purnendu Kumar
  email: purnendu@kumargaur.com
tags: [widgets, links, images, math]
draft: false
---

# Inline Widgets `:[text]{name key=val}`

## Basic widget — name only

A :[badge]{chip} inline.

## Widget with a text label and attributes

The local oscillator :[LO]{tooltip text="Local Oscillator"} drives the mixer.

## Widget with multiple attributes

Signal :[RF]{port dir=in type=analog} enters the block.

## Widget with flag-only attribute (no value)

The :[status]{indicator active} light is green.

## Widget with quoted value containing spaces

:[signal name]{label text="High Frequency Core" class=mono} is the block name.

## Multiple widgets in the same paragraph

:[A]{w1} some text :[B]{w2 x=1} more text :[C]{w3}.

## Widget at start of paragraph

:[START]{marker} and some following text.

## Widget at end of paragraph

Some preceding text and :[END]{marker}.

## Widget — invalid (empty name, must be plain text)

This :[LO]{} should remain as plain text, not a widget.

## Widget — invalid (no braces, must be plain text)

This :[ should remain as plain text.

---

# Link Attributes `[text](url){key=val}`

## Link without attributes

[Standard link](https://example.com) — no attrs field expected.

## Link with single attribute

[Datasheet](./rf.pdf){tooltip=docs}

## Link with multiple attributes

[Download](./file.zip){class=btn download=true rel=noopener}

## Link with quoted attribute value

[Reference](./ref.pdf){title="See the RF docs"}

## Link with attrs AND trailing text (remainder range preserved)

[See more](./more.pdf){class=inline} and this text follows immediately.

## Link without attrs followed by text starting with non-brace

[Plain link](./doc.pdf) {this is not an attr block}

## Link with title in URL parens (title field)

[Titled link](https://example.com "Link title") — title should be captured.

## Link with attrs AND title

[Full link](https://example.com "My title"){class=external}

---

# Image Attributes `![alt](src){key=val}`

## Image without attributes

![Mixer block diagram](./mixer.png)

## Image with single attribute

![Mixer chain](./mixer.png){width=480}

## Image with multiple attributes

![Spectrum plot](./spectrum.png){width=800 height=400 class=diagram}

## Image with quoted attribute

![RF Block](./rf.png){alt="RF block diagram" loading=lazy}

## Image with attrs AND trailing text

![Logo](./logo.svg){class=inline} is our logo.

## Image without attrs, followed by text starting with brace

![Plain image](./img.png) {not an attr block}

---

# Soft-break spanning widget (must be found after text merge)

Some text and :[LO
]{tooltip text="spanning"}
trailing.

---

# Combined: widgets, links, images in one paragraph

The :[mixer]{chip} takes :[RF]{port dir=in} and :[LO]{port dir=in} to produce
:[IF]{port dir=out}. See [the datasheet](./ds.pdf){class=ref} and
![block diagram](./bd.png){width=300}.

---

# Widgets inside directive body

:::note
The :[LO]{tooltip text="Local Oscillator"} is a critical component.

See [datasheet](./lo.pdf){class=ref download=true}.
:::

---

# Math (inline and block)

Inline: $E = mc^2$ and $\alpha + \beta = \gamma$.

Display math block:

$$
\int_0^\infty e^{-x}\,dx = 1
$$

---

# Code spans

Use `const x = 42` and `fn main()` inline.

---

# Emphasis combinations

**Bold**, *italic*, ~~strikethrough~~, ^superscript^, ~subscript~, `code`.

Nested: **bold with *italic inside* here**.

Hard break:  
next line after two trailing spaces.

Soft break:
next line (single newline).
