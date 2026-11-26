# Links

A [normal link](https://github.com/emilk/egui) opens in the browser.

A [link with a title](https://github.com/emilk/egui "The egui repository") shows the title as a tooltip.

An [app link](app://settings) has its own color and tooltip. The app handles the click.

A mention of [Ada](user://ada) has a custom layout.

The build is [passing](status://passing), and the deploy is [failing](status://failing). Each status is
a widget inside the text.

A card is a block widget. It takes rows of its own:

[Release notes](card://release-notes)

A link in a table cell uses the handler too:

| Kind   | Link                              |
|--------|-----------------------------------|
| Normal | [egui](https://github.com/emilk/egui) |
| App    | [Settings](app://settings)        |
