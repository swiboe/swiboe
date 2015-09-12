- Feature Name: layered_rpc
- Start Date: 2015-09-12

# Summary

Everything in Swiboe is a plugin. Many plugins will provide alternative
implementations for similar features, for example one plugin knows how to open
compressed files, another how to open encrypted files and yet another how to
open files on a remote host. For the user, all of this is opaque, she just
"opens". Layered RPCs allows for similar RPCs to have the same name and a well
defined order in which they are tried till one succeeds - transparent to the
user.

This is the fundamental feature of Swiboe - the core only supports this this
layered RPC system. Everything else is implemented in plugins.

# Motivation

Extending the core functionality of Swiboe must allow for swapping in new
implementations for core functions through plugins. Swiboe's abstractions must
therefore be so modular as to deal naturally with this.

Also, frequently many features naturally map to one scarce resource. A prime
example in text editors is the tab key that is used for expanding snippets,
completing words, jumping to next tab stop positions, and inserting a literal
tab character. Given that every one of this feature might be implemented by
another plugin in Swiboe makes it necessary to order which plugin gets the
chance to handle the call. Plugins must also communicate if they handled an RPC
or if another plugin should get a chance to handle it.

# Detailed design

## Registering an RPC

An RPC has a unique `id` and a 16 bit unsigned integer `priority`. The `id`
consists of a dot-separated `name` component and an `implementor` component that
starts with a `:`. So these are valid `id`s.

- `buffer.create:core`
- `on.cursor.moved:ssh_plugin`
- `on.buffer.saved:gui`

The dot parts of `name` are namespaces that separate domains of concern, not
domains of implementation. For example the namespace `buffer` contains all
functions that do something with buffers, no matter which plugin provides the
implementation.

Plugins can register an RPC with an `id` and a `priority` with Swiboe by calling
`core.new_rpc` RPC.

## Calling an RPC

An RPC gets called by a caller either with a full `id` which can only match
one or zero RPCs or with a `selector` that matches an arbitrary number of RPCs.
A `selector` is the prefix of a `name`. For example

- selector `on` matches all RPCs under the `on` namespace, like
  `on.cursor.moved:blub` and `on.file.saved:fileio`, but not
  `onwards.something:someone`.
- selector `on.cursor.moved` matches all RPCs in the `on.cursor.moved` namespace
  like `on.cursor.moved:ssh_plugin`, `on.cursor.moved:gui`,
  `on.cursor.moved.pre:something`.

When Swiboe handles an RPC call, it creates a `PendingRpc` object. This will
exist until the client gets a response from Swiboe.

On creation of a `PendingRpc`, Swiboe will go through the list of registered
RPCs and assemble a list of matching RPCs for the `selector` and store it in the
`PendingRpc`. This list is sorted by priority - higher priority RPCs are tried
first. Swiboe will now start calling the RPCs one by one, see below.

Should an RPC be deleted from Swiboe's list while the `PendingRpc` is alive, it
will skip over it when it is supposed to call it - as if the RPC replied
`ignore` (see next section). If a new matching RPC is registered while the RPC
is running, it will not be called.

Every RPC that returns `handle_partially` or `handle` (see next paragraph) is
allowed to send partial data for the RPC which verbatimely gets relayed to the
caller. Final results from RPCs are aggregated in a list ordered by priority of
the implementors. Once all RPCs have replied, Swiboe returns the list of results
to the caller and deletes the `PendingRpc`. This concludes the RPC.

## A well behaved RPC implementation

An implementation should answer immediately with `handle`, `handle_partially` or
`ignore`. The individual cases mean

- `ignore`: This implementation does not care at all for this request. It
  will not provide any data for the request at all.
- `handle`: This implementation handles the full request, no other plugin should
  get a chance to handle it. Swiboe will not call any further implementations.
- `handle_partially`: This implementation will provide more information about
  this request, but does not fully handle it. Other plugins should get a chance
  as well.

If the implementation does `handle` or `handle_partially` it will start its work
and send zero or more `partial results` (streaming RPCs) and one final result
which can either be `Ok(data)` or an error.

Micro optimizations are possible here: for non-streaming RPCs the `handle` cause
could already contain the `result` so that only one data packet needs to be
sent.

## Data encoding

RPCs can have arbitrary arguments and the data transfer is unstructured -- i.e.
no schema is defined. In most client implementations, the arguments will be
passed as JSON. The implementation of Swiboe right now uses JSON also
internally, to there is no harm in switching that out to another non-structured
encoding.

## Examples

### Example 1: Opening an opaque URI.

This is an example of swapping in new functionality for core functions.

1. Caller calls Swiboe: `"buffer.open" "file://blub.gz"`
    1. Swiboe calls: `"buffer.open:curl_plugin" "file://blub.gz"`
        1. The CURL plugin only fetches stuff from the web, it does not do
           anything with a `file://'`. So it replies: `ignore`
    1. Swiboe calls: `"buffer.open:ssh_plugin" "file://blub.gz"`
        1. The SSH plugin does only handle `ssh://` URI. So it also replies:
           `ignore`.
    1. Swiboe calls: `"buffer.open:gzip_plugin" "file://blub.gz"`
        1. The gzip plugin knows how to handle this, it replies immediately with
           `handled`. It opens the file, unzips its contents and calls:
           `"buffer.create" "with_contents:<file_contents>"`.
        1. It waits for this RPC to complete and returns its success value,
           maybe with additional information (for example the buffer id that
           contains the opened file).
    1. Swiboe returns the succeess value to the client as result of the RPC.
       This ends this RPC call. Swiboe knows many more implementors for this
       RPC, but they will never get called. Especially, the plugin that only
       opens files from disk always runs after the `gzip_plugin`, so it only
       runs if the file is not compressed.

### Example 2: Callbacks

This is an example how the layered RPC system naturally contains a pub/sub
infrastructures through callbacks.

The `cursor` plugin has just moved a cursor in a buffer and wants to inform
every interested party about the change.

1. The `cursor` plugin calls: `"on.cursor.moved"`, passing as arguments the
   cursor id and the new state. It does not wait for this RPC to return, it just
   issues it.
2. Swiboe finds all plugins that registered an RPC that matches
   `"on.cursor.moved"` and calls them in order. All of them should return
   `"ignore"`.

### Example 3: Streaming RPCs & Filtering

We have two implementors for the `list.files` RPC, the implementor `:local`
looks at a local directory, `:ssh` through SSH on a remote host.

1. Caller calls Swiboe: `"list.files" "/tmp"`
    1. The `:local` plugin has higher priority and goes first. It immediately
       replies with `handle_partially`. It then starts to recurse `/tmp` and
       every 20 milliseconds or so, it sends a partial result looking like this
       `{ entries: [ "/tmp/blub", "/tmp/blah", ...] }`. Swiboe will pass these on to
       the caller. Eventually, the recursion end and `:local` will send
       `Error(invalid_permissions)`, because we could not recurse into some
       directories.
    2. Directly after seeing the `handle_partially`, Swiboe will have called
       `:ssh` which behaves similarly to `:local`. But it does not see any
       errors, so it eventually returns `Ok(null)`. The caller gets the partial
       results interleaved without knowledge which implementor sent what.
    3. Swiboe waits for the final result from both implementors before it sends
       the result back to the caller. It will be a list
       `[Error(invalid_permissions), Ok(null)]`.

Now assume that we want to do filtering on the results. We have two implementors
of `list.filter`, one named `:extension` that expects arguments like this:

~~~json
{
    "rpc": {
        "selector": "list.files",
        "args": {
            "directory": "/tmp",
        }
    },
    "extension": ".lua"
}
~~~

And one named `:prefix` that expects arguments like this:

~~~json
{
    "rpc": same_as_above,
    "prefix": "e",
}
~~~

Both will simply call the given RPC and on each partial result filter the
`entries` to only the ones that match and pass them on. By looking at the
arguments, the filters know if the can `handle` or `ignore` the call. You can
even chain them by calling them like this:

~~~json
call "list.filter" {
    "rpc": {
        "selector": "list.filter",
        "args": {
            "rpc": {
                "selector": "list.files",
                "args": {
                    "directory": "/tmp",
                }
            },
            "extension": ".lua"
        }
    },
    "prefix": "e",
}
~~~

# Drawbacks

This RPC system is adding overhead compared to an in-process function call. The
current implementation takes ~75 microseconds for one RPC roundtrip when run
over unix domain socket. Time grows linearly the more plugins are involved in
one RPC.

Most computational expensive work will be done inside of plugins anyways, so the
RPC calls mostly glue the system together. Therefore, so far there is no
evidence that this design is too slow.

# Alternatives

None. Swiboe was build around this concept, it is the big bet.

# Unresolved questions

- Should `core` be called `swiboe` instead?

- Currently there is no way for a RPC to take over any others. For example a
  tracing RPC might be useful that can inspect results and change them before
  returning:<br>
  For example, a caller calls `"blub"`. The first implementor `:trace` with a
  super high priority returns `takeover`. Swiboe interprets that as if it was
  the original caller of the RPC, i.e. it will get the results of all other
  implementors of `"blub"` and has the chance to log their result. Once
  it returns, it will look to the original caller as if `:trace` was the only
  implementor.
