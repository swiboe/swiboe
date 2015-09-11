# Swiboe ¬ [![Build Status](https://travis-ci.org/swiboe/swiboe.svg)](https://travis-ci.org/swiboe/swiboe) [![Join the chat at https://gitter.im/swiboe/swiboe](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/swiboe/swiboe?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

![Swiboe logo](/assets/icon_square/192.png?raw=true])

This is the groundwork the [ideal text
editor](http://www.sirver.net/blog/2015/08/04/the-ideal-text-editor/). So far it
is a design and performance study and an outline of the implementation. It will
gradually merge into a real editor.

My initial thoughts on text editors and my design ideas for this one are on [my
blog](http://sirver.net). The idea is to establish an RFC process for changes
rather quickly though.

The Swiboe community meets on weekdays at 8am CEST in [my twitch
channel](http://www.twitch.tv/sirverii). I stream live commentary of me coding
on Swiboe.

## Philosophy

Everything in Swiboe is a plugin. The core component of Swiboe, the server, has
no concept of what a text editor should do. All it knows about is a protocol that implements a
layered RPC system. All functionality in Swiboe is implemented through this.

So far outlines of the buffer plugin, file completion plugin and several GUI
plugins have been implemented. They are all shells without much functionality
and the driving idea right now is to verify, test and iterate on the design before
committing.

## Getting started

Swiboe only runs on Mac OS X and Linux right now. This is due to it's dependency
[mio](https://github.com/carllerche/mio) not working on Windows and a lack of
contributors for Windows. Mio is working on windows support, once they have it,
we will push for Windows too.

You will need a nightly rust to build the current code. I suggest using
[multirust](https://github.com/brson/multirust) to handle different versions.

After cloning, try building the server and run the tests.

~~~~
$ cargo build --release && cargo test
~~~~

You might want to run the benchmarks too:

~~~~
$ ulimit -n 10000  # Probably only needed on Mac OS X
$ cargo bench
~~~~

Now bring up the server:

~~~~
$ cargo run --bin server --release -- -s /tmp/swiboe.socket
~~~~


Next, in another terminal, try building the terminal GUI and running it:

~~~
$ cd term_gui
$ cargo build --release
$ cargo run --release -- -s /tmp/swiboe.socket
~~~

This drops you in an empty terminal window - welcome to swiboe. Try CTRL + t to
fuzzy search through all the files in the current directory and to open one.

# Contributing

If you are unsure where to start either reach out to me or grep for NOCOM in the
source base - there are many NOCOM (which stand for do-not-commit) while the
project is in outline mode. If you pick one of the NOCOMs you will flesh out the
details of stuff that I have only outlined so far.

If you prefer to work on something bigger picture, you will need to provide some design
proposals first (it would be best to open an issue for discussion).
