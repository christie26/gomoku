import tkinter as tk
from tkinter import font

root = tk.Tk()
root.title("Font Preview")

canvas = tk.Canvas(root)
scrollbar = tk.Scrollbar(root, orient="vertical", command=canvas.yview)
frame = tk.Frame(canvas)

frame.bind("<Configure>", lambda e: canvas.configure(scrollregion=canvas.bbox("all")))

canvas.create_window((0, 0), window=frame, anchor="nw")
canvas.configure(yscrollcommand=scrollbar.set)

fonts = sorted(font.families())

for f in fonts:
    label = tk.Label(frame, text=f"{f} - Sample Text 123", font=(f, 12))
    label.pack(anchor="w")

canvas.pack(side="left", fill="both", expand=True)
scrollbar.pack(side="right", fill="y")

root.mainloop()
