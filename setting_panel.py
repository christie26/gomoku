import tkinter as tk

NAME_FONT = "Rockwell"
LIGHT_BACKGROUND = "#FAEBD7"
BORDER_COLOR = "#6f5c43"


class SettingsPanel:
    def __init__(
        self,
        right_frame,
        on_start_game,
        on_undo,
        on_redo,
        on_debug,
        on_play_mode,
        on_hint,
    ):
        self.is_playing = False
        self.on_start_game = on_start_game
        self.on_undo = on_undo
        self.on_redo = on_redo
        self.on_debug = on_debug
        self.on_hint = on_hint
        self.switch_play_mode = on_play_mode

        self.setting_frame = tk.Frame(right_frame, padx=10, pady=10)
        self.setting_frame.pack(fill="x")
        self.setting_ratios = []

        self.build_ui()

    def get_frame(self):
        return self.setting_frame

    def build_ui(self):
        self.setting_frame.config(
            background=LIGHT_BACKGROUND,
            highlightbackground=BORDER_COLOR,
            highlightcolor=BORDER_COLOR,
            highlightthickness=1,
            relief="solid",
        )
        self.hint_button = tk.Button(
            self.setting_frame,
            text="Hint",
            command=self.on_hint,
            font=NAME_FONT,
            highlightbackground=LIGHT_BACKGROUND,
        )
        self.hint_button.pack(pady=0, padx=0, anchor="w")

        # -------------------
        # 1. Play Mode
        # -------------------
        play_frame = tk.Label(
            self.setting_frame,
            text="Play Mode",
            padx=5,
            pady=0,
            font=(NAME_FONT, 16),
            background=LIGHT_BACKGROUND,
        )
        play_frame.pack(pady=(5, 0), anchor="w")

        self.play_mode = tk.StringVar(value="pvp")

        rb1 = tk.Radiobutton(
            self.setting_frame,
            text="Player vs Player",
            variable=self.play_mode,
            command=self._on_play_mode_change,
            value="pvp",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        rb1.pack(anchor="w")
        self.setting_ratios.append(rb1)

        rb2 = tk.Radiobutton(
            self.setting_frame,
            text="Player vs AI",
            variable=self.play_mode,
            command=self._on_play_mode_change,
            value="pvsa",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        rb2.pack(anchor="w")
        self.setting_ratios.append(rb2)

        rb3 = tk.Radiobutton(
            self.setting_frame,
            text="AI vs Player",
            variable=self.play_mode,
            command=self._on_play_mode_change,
            value="avsp",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        rb3.pack(anchor="w")
        self.setting_ratios.append(rb3)

        # -------------------
        # 2. Ruleset
        # -------------------
        rules_frame = tk.Label(
            self.setting_frame,
            text="Ruleset",
            padx=5,
            pady=0,
            font=(NAME_FONT, 16),
            background=LIGHT_BACKGROUND,
        )
        rules_frame.pack(pady=(15, 0), anchor="w")

        self.ruleset = tk.StringVar(value="1")

        rb4 = tk.Radiobutton(
            self.setting_frame,
            text="Option 1",
            variable=self.ruleset,
            value="1",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        rb4.pack(anchor="w")
        self.setting_ratios.append(rb4)

        rb5 = tk.Radiobutton(
            self.setting_frame,
            text="Option 2",
            variable=self.ruleset,
            value="2",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        rb5.pack(anchor="w")
        self.setting_ratios.append(rb5)

        # -------------------
        # 3. Debug Tool
        # -------------------
        debug_frame = tk.Label(
            self.setting_frame,
            text="Debug Tool",
            padx=5,
            pady=0,
            font=(NAME_FONT, 16),
            background=LIGHT_BACKGROUND,
        )
        debug_frame.pack(pady=(15, 0), anchor="w")

        self.debug_enabled = tk.BooleanVar(value=False)

        self.debug_checkbox = tk.Checkbutton(
            self.setting_frame,
            text="Use Debug Tool",
            variable=self.debug_enabled,
            command=self._on_debug_changed,
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        self.debug_checkbox.pack(anchor="w")

        self.debug_tip = tk.Label(
            self.setting_frame,
            text="Black : max, White : min",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        self.debug_tip.pack(anchor="w")

        # 4. Start Button
        button_frame = tk.Frame(
            self.setting_frame, pady=15, background=LIGHT_BACKGROUND
        )
        button_frame.pack(fill="x")

        self.start_button = tk.Button(
            button_frame,
            text="Start Game",
            command=self.start_game,
            font=NAME_FONT,
            highlightbackground=LIGHT_BACKGROUND,
        )
        self.start_button.pack(pady=3, padx=5, anchor="w")

        # 5. Undo / Redo
        self.undo_button = tk.Button(
            button_frame,
            text="Undo",
            command=self.on_undo,
            state=tk.DISABLED,
            font=NAME_FONT,
            highlightbackground=LIGHT_BACKGROUND,
        )
        self.undo_button.pack(pady=3, padx=5, anchor="w")

        self.redo_button = tk.Button(
            button_frame,
            text="Redo",
            command=self.on_redo,
            state=tk.DISABLED,
            font=NAME_FONT,
            highlightbackground=LIGHT_BACKGROUND,
        )
        self.redo_button.pack(pady=3, padx=5, anchor="w")

    def _on_debug_changed(self):
        value = self.debug_enabled.get()
        self.on_debug(value)

    def _on_play_mode_change(self):
        value = self.play_mode.get()
        self.switch_play_mode(value)

    def start_game(self):
        self.is_playing = True
        self.on_start_game()

    def reset_panel(self, is_playing: bool):
        if is_playing:
            for rb in self.setting_ratios:
                rb.config(state="disabled")
            self.start_button.config(state="disabled")
        else:
            for rb in self.setting_ratios:
                rb.config(state="normal")
            self.start_button.config(state="normal")
