// Denoの必要なモジュールをインポート
import { serveDir } from "https://deno.land/std@0.224.0/http/file_server.ts";
import { WebSocketServer } from "https://deno.land/x/websocket@v0.1.4/mod.ts";

// WebSocketサーバーを作成
const wss = new WebSocketServer(3001);

// "apps"ディレクトリから静的ファイルを提供する
async function handleHttpRequest(req){
  const _url = new URL(req.url);
  
  // "apps"ディレクトリから静的ファイルを提供
  return await serveDir(req, {
    fsRoot: "html",
    urlRoot: "",
  });
}

// HTTPサーバーを起動
const _httpServer = Deno.serve({ port: 3000 }, handleHttpRequest);
console.log(`サーバーが起動しました: http://localhost:3000`);

// WebSocket接続を処理
wss.on("connection", (ws) => {
  console.log("WebSocketの接続が行われました");
  let righttouch = false;

  ws.on("close", () => {
    console.log("WebSocketのせつぞくがきれました");
    clearInterval(pingInterval); // ping送信処理を停止
  });

  ws.on("message", (message) => {
    const messageString = typeof message === "string" ? message : new TextDecoder().decode(message);
    const massages = messageString.split(",");
    console.log(massages[0]);
    const x = 123;
    const y = 345;
    
    if (massages[0] == "lefclick") {
//      robotjs.mouseClick();
      console.log("clicked");
      righttouch = false;
    } else if (massages[0] == "rigclick") {
      if (righttouch == false) {
//        robotjs.mouseClick("right");
        console.log("Rclicked");
      }
      righttouch = true;
    } else if (massages[0] == "cursol") {
//      const mousePos = robotjs.getMousePos();
//      const x = mousePos.x + Number(massages[1]) * 3;
//      const y = mousePos.y + Number(massages[2]) * 3;
//      robotjs.moveMouse(x, y);
      console.log(x + "," + y);
    } else if (massages[0] == "scroll") {
//      const mousePos = robotjs.getMousePos();
//      const x = Number(massages[1]);
//      const y = Number(massages[2]);
//      robotjs.scrollMouse(x, y);
      console.log(x + "," + y);
    } else if (massages[0] == "drag") {
//      const mousePos = robotjs.getMousePos();
//      const x = mousePos.x + Number(massages[1]) * 3;
//      const y = mousePos.y + Number(massages[2]) * 3;
//      robotjs.moveMouse(x, y);
//      robotjs.mouseToggle("down", "left");
      console.log(x + "," + y);
    } else if (massages[0] == "end") {
//      robotjs.mouseToggle("up", "left");
    }
  });

  // 30秒ごとにPingを送信
  const pingInterval = setInterval(() => {
    ws.ping();
  }, 30000);

  ws.on("pong", () => {});
});

// コンソール入力でのシャットダウン処理
const textDecoder = new TextDecoder();
console.log("終了はqまたはｑと入力");

// 標準入力から読み取り
const readLines = async () => {
  for await (const line of Deno.stdin.readable) {
    const input = textDecoder.decode(line).trim();
    if (input === "q" || input === "ｑ") {
      console.log("終了します。");
      httpServer.shutdown();
      Deno.exit(0);
    }
  }
};

readLines();
