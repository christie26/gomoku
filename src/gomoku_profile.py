import cProfile
import pstats
import sys
from pathlib import Path


def main():
    # We run the target script as if it's the __main__ module
    script_path = Path(__file__).parent / "gomoku_gui.py"
    # sys.argv = ["gomoku_gui.py"]  # You can add command-line arguments here if needed
    sys.argv = ["gomoku_gui.py", "--black", "Alice", "--white", "AI"]

    exec(compile(script_path.read_text(), str(script_path), "exec"), globals())


if __name__ == "__main__":
    profiler = cProfile.Profile()
    profiler.enable()

    main()

    profiler.disable()
    stats = pstats.Stats(profiler)
    stats.strip_dirs()
    stats.sort_stats(pstats.SortKey.CUMULATIVE)
    stats.print_stats(50)  # Print top 50 cumulative time functions

    stats.dump_stats("gomoku.prof")
