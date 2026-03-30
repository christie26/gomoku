import tkinter as tk

NAME_FONT = "Rockwell"
LIGHT_BACKGROUND = "#FAEBD7"
BORDER_COLOR = "#6f5c43"


class SettingsPanel:
    def __init__(self, right_frame, on_start_game, on_undo, on_redo):
        self.is_playing = False
        self.on_start_game = on_start_game
        self.on_undo = on_undo
        self.on_redo = on_redo

        self.setting_frame = tk.Frame(right_frame, padx=10, pady=10)
        self.setting_frame.pack(fill="x")
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

        # -------------------
        # 1. Play Mode
        # -------------------
        play_frame = tk.Label(
            self.setting_frame,
            text="Play Mode",
            padx=5,
            pady=5,
            font=(NAME_FONT, 16),
            background=LIGHT_BACKGROUND,
        )
        play_frame.pack(pady=5, anchor="w")

        self.play_mode = tk.StringVar(value="pvp")

        tk.Radiobutton(
            self.setting_frame,
            text="Player vs Player",
            variable=self.play_mode,
            value="pvp",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
            tags="setting",
        ).pack(anchor="w")

        tk.Radiobutton(
            self.setting_frame,
            text="Player vs AI",
            variable=self.play_mode,
            value="pvsa",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
            tags="setting",
        ).pack(anchor="w")

        tk.Radiobutton(
            self.setting_frame,
            text="AI vs Player",
            variable=self.play_mode,
            value="avsp",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
            tags="setting",
        ).pack(anchor="w")

        # -------------------
        # 2. Ruleset
        # -------------------
        rules_frame = tk.Label(
            self.setting_frame,
            text="Ruleset",
            padx=5,
            pady=5,
            font=(NAME_FONT, 16),
            background=LIGHT_BACKGROUND,
        )
        rules_frame.pack(pady=5, anchor="w")

        self.ruleset = tk.StringVar(value="1")

        tk.Radiobutton(
            self.setting_frame,
            text="Option 1",
            variable=self.ruleset,
            value="1",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        ).pack(anchor="w")

        tk.Radiobutton(
            self.setting_frame,
            text="Option 2",
            variable=self.ruleset,
            value="2",
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        ).pack(anchor="w")

        # -------------------
        # 3. Debug Tool
        # -------------------
        debug_frame = tk.Label(
            self.setting_frame,
            text="Debug Tool",
            padx=5,
            pady=5,
            font=(NAME_FONT, 16),
            background=LIGHT_BACKGROUND,
        )
        debug_frame.pack(pady=5, anchor="w")

        self.debug_enabled = tk.BooleanVar()

        self.debug_checkbox = tk.Checkbutton(
            self.setting_frame,
            text="Use Debug Tool",
            variable=self.debug_enabled,
            font=NAME_FONT,
            background=LIGHT_BACKGROUND,
        )
        self.debug_checkbox.pack(anchor="w")

        # -------------------
        # 4. Start Button
        # -------------------
        self.start_button = tk.Button(
            self.setting_frame, text="Start Game", command=self.start_game
        )
        self.start_button.pack(fill="x", pady=5)

        # -------------------
        # 5. Undo / Redo
        # -------------------
        action_frame = tk.Frame(self.setting_frame)
        action_frame.pack(fill="x", pady=5)

        self.undo_button = tk.Button(
            action_frame, text="Undo", command=self.on_undo, state=tk.DISABLED
        )
        self.undo_button.pack(fill="x")

        self.redo_button = tk.Button(
            action_frame, text="Redo", command=self.on_redo, state=tk.DISABLED
        )
        self.redo_button.pack(fill="x")

    # -------------------
    # Game State Control
    # -------------------
    def start_game(self):
        self.is_playing = True
        self.update_controls_state()
        self.on_start_game()

    def update_controls_state(self):
        state = "disabled" if self.is_playing else "normal"

        # Disable play mode + ruleset
        for widget in self.setting_frame.winfo_children():
            for child in widget.winfo_children():
                if isinstance(child, tk.Radiobutton):
                    child.config(state=state)

        # Debug tool should stay enabled
        self.debug_checkbox.config(state="normal")

        # Optional: disable start button during game
        self.start_button.config(state=state)
