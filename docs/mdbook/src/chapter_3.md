# Chapter 3

Another sample chapter for style testing.

## Section One

Body text goes here. Observer collects metrics and sends them to your server endpoint.

### Nested Section

More detail with an inline `variable` and some **bold** and _italic_ text.

## Code Block

```rust
fn main() {
    println!("Observer starting...");
}
```

## Another Table

| Key             | Type   | Required | Description          |
|-----------------|--------|----------|----------------------|
| `api_key`       | string | yes      | Authentication key   |
| `metric_secs`   | int    | no       | Collection interval  |
| `speedtest_secs`| int    | no       | Speedtest interval   |

> **Note:** All interval values must be within the allowed range defined in the config.

## List Example

- First item in a list
- Second item with `inline code`
- Third item with a [link](./chapter_1.md)
