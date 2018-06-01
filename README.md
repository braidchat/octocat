# Octocat #

A Braid bot for interacting with Github issues

Building with rustc 1.12.0-nightly (27e766d7b 2016-07-19)

To set up:

  - Generate an access token with `repo` scope at [https://github.com/settings/tokens](https://github.com/settings/tokens)
  - Add a webhook on Github from the relevant repository (from repo Settings), with the triggered events "Issues" and "Issue Comment"
  - Add the bot on Braid, with the path of webhook url being `/message`


Example conf.toml:

```
[general]
port = "7777"
db_name = "octocat_db.sqlite"

[braid]
name = "octocat"
api_url = "https://api.braid.chat"
site_url = "https://braid.chat"
app_id = "app id from braid"
token = "app token from braid"

[github]
webhook_secret = "random secret you put in the github webhook conf"

[[repos]]
token = "token created from github"
org = "jamesnvc"
repo = "dotfiles"
tag_id = "some braid tag id"

[[repos]]
token = "token created from github"
org = "jamesnvc"
repo = "emacs.d"
tag_id = "some braid tag id"
```

## Octocat in Action

![Bot running demo](https://s3.amazonaws.com/chat.leanpixel.com/uploads/579c1378-7d27-4454-8864-738df842d6fa/demo2.gif)
