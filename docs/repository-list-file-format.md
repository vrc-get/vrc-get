# Repository List file format

This document describes the format of the repository list file used in ALCOM / vrc-get.

## File format

1. The file is a UTF-8 encoded text file.
2. For each line, text after '#' will be ignored as a comment.
3. Each line contains a repository URL, or empty.
4. Each line is trimmed before processing. (this mean any line only with spaces will be ignored)
5. Each repository line should only contain a valid URL.
6. The URL schema must be `http`, `https`, or `vcc`.\ 
   Other schemas are ignored and might be recognized for other purposes in the future.
7. If the URL is a `http` or `https` URL, the URL represents a VPM repository without headers.
8. If the URL is a `vcc` URL, the URL should be a VCC URL to add VPM repository, which is described below.\
   This notation is used to express the repository with headers.

## VCC URL format

The VCC URL to add VPM Repository will be a valid URL with the following format:

- schema must be `vcc`
- the host part must be `vpm`
- the path part must be `/addRepo`
- the query part must contain single `url` parameter which represents the repository URL to add.
- the query part may contain `headers[]` parameter with represents the HTTP headers for the repository.
  - query value will be split by `:` and prior part will be the header name and the rest will be the header value.

## Examples

```text
# This is a comment
http://example.com/repo
https://example.com/repo

vcc://vpm/addRepo?url=http://example.com/repo&headers[]=header-name:header-value
```

This file represents a repository list with the following repositories:

- `http://example.com/repo`
- `https://example.com/repo`
- `http://example.com/repo` with a custom header `header-name:header-value`

Another example may be found at the `repositories.txt` at the root of this repository.
