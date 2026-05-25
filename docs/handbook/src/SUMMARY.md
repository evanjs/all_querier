# Summary

The purpose of this application is to provide a centralized utility to search across a variety of services
Once the search queries are retrieved, minimal processing it performed before presenting the data back to the querying user.

There are currently two (interactive) methods by which users can utilize this system

1. CLI utility
2. Nushell Plugin

The CLI utility can be forced to provide `json` output via the `--json` flag
For the Nushell plugin, the data, so long as it is in a format that is natively compatible with Nushell, is sent to Nushell as a Nushell Record
Doing so ensures that Nushell can tabulate the data without any extra work

The order of the colums from the plugin and CLI differ, but the data provided is the same

When using Nushell with either of the above approaches, the following two commands should be functionally equivalent:

```nushell
search -p anilist -t anime 'one piece'
  | move id label description item_type data --after provider
...
allq-cli.exe search --json -p anilist -t anime 'one piece' | from json
```

This book details the structure of the application and its constituent providers