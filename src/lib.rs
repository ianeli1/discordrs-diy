use std::collections::HashMap;
use std::sync::Arc;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    Error,
};

pub struct Bot {
    token: &'static str,
    options: BotOptions,
    client: Client,
    uses_prefix: bool, //yes if prefix, false if suffix
    commands: HashMap<String, Command>,
}

pub struct Command {
    reply: fn(&Context, &Message, args: &str) -> String, //TODO allow to send message objects or somn
}

impl TypeMapKey for Bot {
    type Value = Arc<Bot>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let bot = Bot::from_context(&ctx).await;
        if let Err(_bot_error) = bot {
            todo!() //FIXME
        }

        let bot = bot.unwrap();
        //find if suffix or prefix is present

        let raw_content = msg.content.clone();
        let content = raw_content.trim();

        let trigger: &str;

        let first_word = match content.split(" ").next() {
            Some(word) => word,
            None => return,
        };
        //extract main trigger without prefix or suffix
        let prefix_length = bot.options.prefix.len();

        if bot.uses_prefix && first_word.starts_with(bot.options.prefix) {
            trigger = &first_word[prefix_length..]
        } else if !bot.uses_prefix && content.ends_with(bot.options.suffix) {
            trigger = first_word;
        } else {
            return;
        }

        //extract arguments without trigger, prefix or suffix

        let mut args = content.clone();

        //remove suffix or prefix
        if bot.uses_prefix {
            args = &args[prefix_length..]
        } else {
            let new_finish = args.len() - bot.options.suffix.len();
            args = &args[..new_finish].trim_end();
        }

        {
            //remove trigger
            let trigger_len = trigger.len();
            args = args[trigger_len..].trim_start();
        }

        //check if there's a command that matches

        let command = match bot.commands.get(trigger) {
            Some(command) => command,
            None => return,
        };

        //run that command

        let reply_result = (command.reply)(&ctx, &msg, &args);

        //reply and/or react with the result

        if let Err(why) = msg.channel_id.say(&ctx.http, reply_result).await {
            println!("An error ocurred {:?}", why)
        }
    }

    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        println!("{} is connected!", data_about_bot.user.name);
    }
}

pub struct BotOptions {
    prefix: &'static str,
    suffix: &'static str,
    ignore_caps: bool,
}

impl Bot {
    pub async fn new(token: &'static str, mut options: BotOptions) -> Result<Arc<Bot>, Error> {
        let default_intents: GatewayIntents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let client = Client::builder(token, default_intents)
            .event_handler(Handler)
            .await?;

        let uses_prefix = options.prefix.len() > 0;

        if options.ignore_caps {
            if uses_prefix {
                let new_prefix = options.prefix.to_lowercase();
                options.prefix = Box::leak(new_prefix.into_boxed_str());
            } else {
                let new_suffix = options.suffix.to_lowercase();
                options.suffix = Box::leak(new_suffix.into_boxed_str());
            }
        }

        let bot = Arc::new(Bot {
            client,
            token,
            options,
            uses_prefix,
            commands: HashMap::new(),
        });

        {
            let mut data = bot.client.data.write().await;
            //insert bot into context
            data.insert::<Bot>(Arc::clone(&bot));
        }

        return Ok(bot);
    }

    pub async fn from_context(ctx: &Context) -> Result<Arc<Bot>, Error> {
        let data = ctx.data.write().await;
        return match data.get::<Bot>() {
            Some(bot) => Ok(Arc::clone(bot)),
            None => Err(Error::Other("Bot not present in context")),
        };
    }

    fn on(&mut self, pat: &str, command: Command) {
        let trigger: String;
        if self.options.ignore_caps {
            trigger = pat.to_lowercase().to_string();
        } else {
            trigger = pat.to_string();
        }
        self.commands.insert(trigger, command);
    }
}
