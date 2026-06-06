---
title: Edge Cases Sample
---

# Feature completeness test

This file tests markdown elements that might not be heavily used but are supported by the AST.

## Horizontal Rules

Here is a horizontal rule:

---

And another one:

***

## Task Lists

- [ ] Incomplete task
- [x] Complete task
- [ ] Another incomplete task

## HTML Blocks

Below is an HTML block:

<div class="custom-widget" data-id="123">
  <span>Raw HTML content</span>
</div>

And some inline HTML: <span style="color: red;">Red text</span>

## Definition Lists

Apple
:   Pomaceous fruit of plants of the genus Malus in
    the family Rosaceae.

Orange
:   The fruit of the citrus species Citrus × sinensis in the family Rutaceae.
