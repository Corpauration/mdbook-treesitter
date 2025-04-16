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
# This would no longer be necessary
# [output.html]
# additional-js = ["treesitter.js"]

[preprocessor.treesitter]
command = "mdbook-treesitter"
languages = ["javascript"]
```

<!-- This would no longer be necessary -->
<!-- Add this javascript in the file `treesitter.js` at the root of your project: -->

<!-- ```javascript -->
<!-- let t = document.getElementsByClassName("language-treesitter"); -->
<!-- for (let i = 0; i < t.length; i++) { -->
<!--   t[i].innerHTML = t[i].innerText; -->
<!-- } -->
<!-- ``` -->

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
- Next, create a folder `language_name` in the `tree-sitter` folder
- Finally, add in it your scm files

Example for javascript:

```
- My awesome mdBook/
    - book.toml
    - treesitter.js
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
