"""HTTP server: accepts a raw image POST and returns the parsed board as JSON."""
import os
import tempfile

from flask import Flask, request

from detector import parse_board, render_board

app = Flask(__name__)


@app.route("/health")
def health():
    return "ok"


@app.route("/parse", methods=["POST"])
def parse():
    with tempfile.NamedTemporaryFile(suffix=".jpg", delete=False) as f:
        f.write(request.data)
        tmp = f.name
    try:
        board = parse_board(tmp)
        return render_board(board), 200, {"Content-Type": "text/plain; charset=utf-8"}
    except Exception as e:
        return str(e), 500, {"Content-Type": "text/plain; charset=utf-8"}
    finally:
        os.unlink(tmp)


if __name__ == "__main__":
    port = int(os.environ.get("PORT", 5000))
    app.run(host="0.0.0.0", port=port)
