---
title: Frontmatter Types Coverage
category: test
tags:
  - frontmatter
  - yaml
author:
  name: Test Author
  email: test@example.com
draft: false
weight: 42
ratio: 3.14
date: 2026-07-16
version: "1.2.0"
aliases: [/old-path, /another-old-path]
keywords: [markdown, ast, parser]
nested:
  level: 2
  active: true
  score: 9.5
---

# Frontmatter Types

This file exercises all YAML frontmatter value types that the mini-parser must handle.

The frontmatter above contains:

- `title` — plain unquoted string
- `category` — plain string
- `tags` — block-style list of strings
- `author` — nested object with `name` and `email`
- `draft` — boolean false
- `weight` — integer
- `ratio` — float
- `date` — date string (must be kept as string, not converted)
- `version` — quoted string (contains a dot — ensure not parsed as number)
- `aliases` — flow-style list `[/old-path, /another-old-path]`
- `keywords` — flow-style list
- `nested` — nested object with mixed types

## Body content

Normal paragraph to ensure the body is parsed correctly after frontmatter.

A second paragraph with **strong** and *emphasis*.
