# wlr-which-key

Keymap manager for wlroots-based compositors. Inspired by [which-key.nvim](https://github.com/folke/which-key.nvim).

## Installation

### From Source

```
git clone https://github.com/MaxVerevkin/wlr-which-key
cd wlr-which-key
cargo install --path . --locked
```

## Configuration

Config file: `$XDG_CONFIG_HOME/wlr-which-key/config.yaml` or `~/.config/wlr-which-key/config.yaml`.

Example config:

```yaml
font: JetBrainsMono Nerd Font 12
background: "#282828d0"
anchor: bottom-right
margin_right: 30
margin_bottom: 30
separator: " ➜ "

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

![image](https://user-images.githubusercontent.com/34583604/229412213-221dd462-e72a-43da-8066-1e81d04b3b48.png)

![image](https://user-images.githubusercontent.com/34583604/229412221-33e347d5-d86a-49be-96bd-0fe669c4b871.png)
