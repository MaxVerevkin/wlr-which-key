# wlr-which-key

Keymap manager for wlroots-based compositors. Inspired by [which-key.nvim](https://github.com/folke/which-key.nvim).

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/wlr-which-key.svg)](https://repology.org/project/wlr-which-key/versions)

### From Source

```sh
cargo install wlr-which-key --locked
```

## Configuration

Default config file: `$XDG_CONFIG_HOME/wlr-which-key/config.yaml` or `~/.config/wlr-which-key/config.yaml`. Run `wlr-which-key --help` for more info.

Keybindings may be single characters (e.g. `a`, `B`) or [xkb key labels](https://github.com/xkbcommon/libxkbcommon/blob/master/include/xkbcommon/xkbcommon-keysyms.h) (without the `XKB_KEY_` prefix, e.g. `Return`, `Insert`). Ctrl and Alt modifiers are supported (like `Ctrl+Return` or `Ctrl+Alt+a`).

Example config:

```yaml
# Theming
font: JetBrainsMono Nerd Font 12
background: "#282828d0"
color: "#fbf1c7"
border: "#8ec07c"
separator: " ➜ "
border_width: 2
corner_r: 10
padding: 15 # Defaults to corner_r

# Anchor and margin
anchor: center # One of center, left, right, top, bottom, bottom-left, top-left, etc.
# Only relevant when anchor is not center
margin_right: 0
margin_bottom: 0
margin_left: 0
margin_top: 0

menu: 
  "w":
    desc: WiFi
    submenu:
      "t": { desc: Toggle, cmd: wifi_toggle.sh }
      "c": { desc: Connections, cmd: kitty --class nmtui-connect nmtui-connect }
  "p":
    desc: Power
    submenu:
      "s": { desc: Sleep, cmd: systemctl suspend }
      "r": { desc: Reboot, cmd: reboot }
      "o": { desc: Off, cmd: poweroff }
  "t":
    desc: Theme
    submenu:
      "d": { desc: Dark, cmd: dark-theme on }
      "l": { desc: Light, cmd: dark-theme off }
      "t": { desc: Toggle, cmd: dark-theme toggle }
  "l":
    desc: Laptop Screen
    submenu:
      "t": { desc: Toggle On/Off, cmd: toggle-laptop-display.sh }
      "s":
        desc: Scale
        submenu:
          "1": { desc: Set Scale to 1.0, cmd: wlr-randr --output eDP-1 --scale 1 }
          "2": { desc: Set Scale to 1.1, cmd: wlr-randr --output eDP-1 --scale 1.1 }
          "3": { desc: Set Scale to 1.2, cmd: wlr-randr --output eDP-1 --scale 1.2 }
          "4": { desc: Set Scale to 1.3, cmd: wlr-randr --output eDP-1 --scale 1.3 }
```

![image](https://user-images.githubusercontent.com/34583604/233025292-af0d5798-1854-4809-b08f-2e8f1a65b3ce.png)

![image](https://user-images.githubusercontent.com/34583604/233025368-e59a386a-6a52-4168-a6e3-5102ea6329cf.png)
