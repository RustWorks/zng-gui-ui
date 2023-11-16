# TextInput

* Touch selection.
    - Test with RTL and bidirectional text.
    - Context menu appears when selecting (or just interacting, if it's an editable field)
        - Not a normal context menu, "floating toolbar"?
    - Implement `touch_carets` touch drag.
        - Implement in the layered shape?
        - Hit-test area full shape rectangle.

* Implement IME.
    - Add event:
        - https://docs.rs/winit/latest/winit/event/enum.Ime.html
    - Add API: 
        - https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_ime_cursor_area
        - https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_ime_allowed

* Implement `obscure_txt`.
    - Just replace chars before segmenting?
    - Firefox handles composite emoji like 👩🏽‍🎤 weirdly, one char per utf-8 char, but some are selected together.
        - Chrome also shows one `•` per char, but they all select together.
        - Chrome is better, but both indicate that the actual text is segmented.
        - WPF always edits per-char, even for emoji that is rendered as a single glyph.
        - Flutter behaves like Firefox in what is selected together (plus the caret does not look well positioned).
        - Firefox, Chrome and Flutter all use the real text segments for edit operations, Firefox and Flutter only have some
          bug editing composite emoji.
    - Lets implement substitution in `ShapedText`?
        - Must be all in a single line.
        - Can wrap?
        - Must be implemented in `ShapedTextBuilder::push_text`, to preserve mapping to segments.
    - What about accessibility, is the password shared with screen-readers?
        - They do not, only reads "star" for each char typed.
        - So we should not share the password text?
            - Just generate a "•••" text?

# Accessibility

* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen).
    - `nestor`: All our nodes and properties are nesting constructors. (name already taken)
    - `ctorx`: Constructor/Context.
    - `xctor`: Context/Constructor.
    - `xnest`: Context nesting.
    - `nestx`: Nesting context.
    - `nestc`: Nesting constructor. 
    - `nestcx`, `cxnest`.
    - `nidulus` or `nidula`: Small nest fungus name. +Fungus related like Rust, -Fungus disguised as a bird nest, not related with our
    nesting stuff.

* Review all docs.
* Review prebuild distribution.
* Pick license and code of conduct.
* Create a GitHub user for the project?
* Create issues for each TODO.

* Publish (after all TODOs in this file resolved).
* Announce in social media.

* After publish only use pull requests.
    - We used a lot of partial commits during development.
    - Is that a problem in git history?
    - Research how other projects handled this issue.