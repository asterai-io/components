# asterai:cli components

Shared interface: `asterai:cli/command`
```wit
run: func(args: string, stdin: option<string>) -> result<string, string>;
```

File operations use the `asterai:fs` interface as the storage backend.

## Tools

| Component                | Description                                    | Priority | CRUD role     |
|--------------------------|------------------------------------------------|----------|---------------|
| **File operations**      |
| `ls`                     | List files and directories                     | High     | Read          |
| `cat`                    | Read file contents                             | High     | Read          |
| `cp`                     | Copy files/directories                         | High     | Create        |
| `mv`                     | Move/rename files                              | High     | Update        |
| `rm`                     | Remove files/directories                       | High     | Delete        |
| `mkdir`                  | Create directories                             | High     | Create        |
| `touch`                  | Create empty file / update timestamp           | Medium   | Create        |
| **Text processing**      |
| `grep`                   | Search content by pattern                      | High     | Read          |
| `sed`                    | Find/replace in text                           | High     | Update        |
| `awk`                    | Column extraction and text processing          | Medium   | Read          |
| `jq`                     | Parse, query, and transform JSON               | High     | Read/Update   |
| `head`                   | First N lines of input                         | Medium   | Read          |
| `tail`                   | Last N lines of input                          | Medium   | Read          |
| `sort`                   | Sort lines                                     | Low      | Read          |
| `uniq`                   | Deduplicate consecutive lines                  | Low      | Read          |
| `wc`                     | Count lines, words, characters                 | Low      | Read          |
| `cut`                    | Extract fields/columns by delimiter            | Low      | Read          |
| `tr`                     | Translate/replace characters                   | Low      | Read          |
| **Search & navigation**  |
| `find`                   | Search for files by name/pattern               | High     | Read          |
| `tree`                   | Display directory structure                    | Medium   | Read          |
| **File content writing** |
| `tee`                    | Write stdin to file and pass through to stdout | Medium   | Create/Update |
| **File info**            |
| `diff`                   | Compare two files or strings                   | Medium   | Read          |
| `stat`                   | File metadata (size, timestamps, permissions)  | Low      | Read          |
