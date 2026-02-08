# asterai:firecrawl

A component for scraping webpages and searching the web using the [Firecrawl](https://firecrawl.dev) API.

```wit
scrape: func(url: string) -> string;
search: func(query: string, limit: u32) -> string;
```

`scrape` returns the page content as markdown.
`search` returns a JSON array of results, each with `url`, `title`, and `description` fields.

Requires the `FIRECRAWL_KEY` environment variable.
