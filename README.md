Super simple window manager in Rust.

Accronym TBD.

If you're wondering why any of this then look -- `afwm [-y|--why]`

LOC count (according to `loc` tool): `950`

I am Rust noob. I started this a few days ago (heavily inspired by
Lanta at first). I have never touched X before this. Pls go easy on my
fragile existence.

# Todos

- refactor much code and contain repeated logic in Desktop possibly?

- tiling mode
  - window gaps in tiling mode

- fix focusing issues (only gain focus when MOD+click on window again)

- fix full screen windows breaking EVERYTHING

- fix some windows unable to be moved/resized

- window cycle when highlighting over some ignores them during shuffle
  (because on MOD key press it refocuses then performs focus change again)

- set cursor on start

- status bar with workspace info

- window borders

- dmenu coloring matching status bar coloring

- properly comment (add where missing, fix old comments)

- OPTIMISE