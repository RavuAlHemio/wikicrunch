# wikicrunch

Attempts to crunch a Wikipedia article dump from wikitext to plaintext, which can then theoretically be used to train a neural network.

Consists of a Rust and a PHP part. The Rust part parses the XML dump and feeds the wikitext of each article to the PHP part, which uses Wikimedia's Parsoid to convert the wikitext to XHTML, returning it to the Rust part which then traverses the XHTML and reduces it to plaintext.

The Rust and the PHP part communicate over an overly simplistic TCP-based IPC protocol.
