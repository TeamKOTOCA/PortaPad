const robot = require('robotjs');
const readline = require('readline');
const robot = require('robotjs');

let righttouch = false;
let scrolled = 0;

function getmessage(message) {
  try{
    const messageString = message.data;
    const massages = messageString.split(",");
    console.log(massages[0]);
    
    if (massages[0] == "lefclick") {
      robot.mouseClick();
      console.log("clicked");
      righttouch = false;
    } else if (massages[0] == "rigclick") {
      if (righttouch == false) {
        robot.mouseClick('right');
        console.log("Rclicked");
      }
      righttouch = true;
    } else if (massages[0] == "cursol") {
      let mousePos = [0, 0];
        mousePos = robot.getMousePos();
        console.log(mousePos);
      if (mousePos != null && massages.length >= 1) {
        const x = mousePos[0] + Number(massages[1]) * 4;
        const y = mousePos[1] + Number(massages[2]) * 4;
        console.log(x + "," + y);
        robot.moveMouse(x, y);
      }
    } else if (massages[0] == "scroll") {
      if (scrolled >= 3) {
        const mousePos = robot.getMousePos();
        const x = Number(massages[1]);
        const y = Number(massages[2]);
        robot.scrollMouse(x,y);
        console.log(x + "," + y);
        
        scrolled = 0;
      }
      scrolled++;
    } else if (massages[0] == "drag") {
      robot.ChangeMouse(1,1);
      let mousePos = [0, 0];
      mousePos = robot.GetMouse();
      console.log(mousePos);
      if (mousePos != null && massages.length >= 1) {
        const x = mousePos[0] + Number(massages[1]) * 4;
        const y = mousePos[1] + Number(massages[2]) * 4;
        console.log(x + "," + y);
        robot.MoveMouse(x, y);
      }
    } else if (massages[0] == "end") {
      robot.ChangeMouse(1,0);
      robot.ChangeMouse(3,0);
    }
  }catch(e){
    console.error(e);
  }
}

//console上での終了操作

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});
console.log('終了はqまたはｑと入力');

rl.on('line', (input) => {
    if (input.trim() === 'q' || input.trim() === "ｑ") {
        console.log('終了します。');
        rl.close();
        process.exit(0);
    }
});
