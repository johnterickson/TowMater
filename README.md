# Rust Bot
A template bot made in Rust for Rocket League made with the [RLBot framework](https://github.com/RLBot/RLBot).

## Setup and run
- Install Rust and Cargo.
- Install [RLBotGUI](http://rlbot.org/). See the [install video](https://www.youtube.com/watch?v=oXkbizklI2U) for guidance.
- Run RLBotGUI, press "Add > Load Cfg File" and select `rustbot_dev/rustbot.cfg`. The bot should now appear in the GUI.
- Add the bot to a team and start a match. Rocket League should open and immediately start a match with the bot.

## Distribution
When the bot is ready to be distributed (for example, submitted to a tournament), run `python package.py` to create a .zip file you can submit.
