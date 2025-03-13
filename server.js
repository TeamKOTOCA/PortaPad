const express = require('express');
const path = require('path');
const http = require('http');//httpsサーバー
const WebSocket = require('ws');
const robot = require('robotjs');

const app = express();
const server = http.createServer(app); // HTTPサーバー作成
const ws = new WebSocket.Server({ server }); // WebSocketをHTTPサーバーに統合

// /静的ファイルを公開 (http://localhost:3000/)
app.use('/', express.static(path.join(__dirname, 'html')));

// WebSocketの接続処理
ws.on('connection', (ws, req) => {
    console.log('WebSocket connected');
        ws.on('close', () => {

            console.log('WebSocket disconnected');
        });

        ws.on('message', (message) => {
            const messageString = message.toString();
            let points = messageString.split(',');
            robot.moveMouse(points[0], points[1]);
            console.log(messageString);
        })

        // 30秒ごとにPingを送る
        const pingInterval = setInterval(() => {
            ws.ping(); // クライアントにPingを送信
        }, 30000); // 30秒おきにPing

        // クライアントからPongを受信したとき
        ws.on('pong', () => {});
});


//サーバーを起動
const PORT = 3000;
server.listen(PORT, () => {
    console.log(`Server is started at http://localhost:${PORT}`);
});


//console上での終了操作
const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});
console.log('終了はqと入力');

rl.on('line', (input) => {
    if (input.trim() === 'q' || input.trim() === "ｑ") {
        console.log('終了します。');
        rl.close();
        process.exit(0);
    }
});