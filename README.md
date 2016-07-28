# Octocat #

A Braid bot for interacting with Github issues

Building with rustc 1.12.0-nightly (27e766d7b 2016-07-19)

To set up:

  - Generate an access token with `repo` scope at [https://github.com/settings/tokens](https://github.com/settings/tokens)
  - Add a webhook on Github from the relevant repository (from repo Settings)


Example conf.toml:

```
[general]
port = "7777"

[braid]
name = "octocat"
api_url = "https://api.braid.chat/bots/message"
site_url = "https://braid.chat"
app_id = "app id from braid"
token = "app token from braid"
group_id = "bot group id"

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
