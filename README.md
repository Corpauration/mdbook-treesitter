# mdbook-treesitter

mdbook-treesitter is an [mdBook](https://github.com/rust-lang-nursery/mdBook) preprocessor for html adding [tree-sitter](https://tree-sitter.github.io/tree-sitter/) highlighting support.

It simply translates the tree-sitter highlighting into highlightjs one.

## Installation

Install the preprocessor:

```shell
cargo install mdbook-treesitter
```

### Configure mdBook

Add this in your `book.toml`:

```toml
[preprocessor.treesitter]
command = "mdbook-treesitter"
languages = ["javascript"]
```

## Usage

Use usual codeblocks like that:

````markdown
```javascript
console.log(this.a + b + "c" + 4);
```
````

Wait, you need to add related tree-sitter files:

- Create a folder `treesitter` in the root of your mdBook project
- Then, add your `language_name.so` in the created folder
  - Note: This also works on Windows systems, copy the tree-sitter `parser.dll` as `language_name.so`, even if it is not a `*.so` file.
- Next, create a folder `language_name` in the `tree-sitter` folder
- Finally, add in it your scm files

Example for javascript:

```
- My awesome mdBook/
    - book.toml
    - book/
    - src/
    - treesitter/
        - javascript.so
        - javascript/
            - highlights.scm
            - injections.scm
            - locals.scm
```


&nbsp;
&nbsp;

🧃
