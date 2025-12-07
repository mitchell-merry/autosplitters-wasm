# cuphead

This is a Rust port of the C# component that was used for 8+ years: https://github.com/ShootMe/LiveSplit.Cuphead.

The improvements here are:

- written using the auto-splitting runtime, which is cross-platform (so supports both Windows and Mac)
- does not hardcode signatures or offsets, and gets pointer paths by using debug symbols from the mono dll in memory
  (more resilient to updates (lol), easier to reason about and write)
- more features (since it is the currently supported autosplitter for the community)
- vastly simplified settings structure

## Contributing

Largely see the parent README.

I have a brain dump here, if you prefer video format: https://youtu.be/ly9r0Hd2CnY (links in description)