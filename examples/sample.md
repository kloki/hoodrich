# hoodrich

Markdown to **ratatui** text, with *emphasis*, ~~strikethrough~~ and `inline code`.
A [link](https://github.com/kloki/hoodrich) and an entity: fish &amp; chips.

## Lists

- first item
- second with **bold**
  - nested item
- [ ] open task
- [x] done task

1. one
2. two

### Quotes

> A quote that spans
> two lines.
>
> > And a nested one.

Setext heading
--------------

#### Code

```rust
/// Adds two numbers.
fn add(a: i32, b: i32) -> i32 {
    let total = a + b; // sum
    total
}
```

```python
def greet(name: str) -> None:
    print(f"hello {name}")
```

    indented code block

---

| Name  | Score | Note       |
| :---- | ----: | ---------- |
| Ada   |    10 | `fast`     |
| Linus |     9 | **strong** |

A footnote reference[^1] and a hard break\
on the next line.

[^1]: The footnote text.

<details>html block</details>
