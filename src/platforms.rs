use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Platform {
    pub name: &'static str,
    /// Format with one {} for the username
    pub profile_url: &'static str,
}

pub fn all_platforms() -> Vec<Platform> {
    vec![
        Platform {
            name: "GitHub",
            profile_url: "https://github.com/{}",
        },
        Platform {
            name: "GitLab",
            profile_url: "https://gitlab.com/{}",
        },
        Platform {
            name: "Reddit",
            profile_url: "https://www.reddit.com/user/{}",
        },
        Platform {
            name: "X (Twitter)",
            profile_url: "https://x.com/{}",
        },
        Platform {
            name: "Instagram",
            profile_url: "https://www.instagram.com/{}/",
        },
        Platform {
            name: "TikTok",
            profile_url: "https://www.tiktok.com/@{}",
        },
        Platform {
            name: "YouTube",
            profile_url: "https://www.youtube.com/@{}",
        },
        Platform {
            name: "LinkedIn",
            profile_url: "https://www.linkedin.com/in/{}",
        },
        Platform {
            name: "Pinterest",
            profile_url: "https://www.pinterest.com/{}/",
        },
        Platform {
            name: "Twitch",
            profile_url: "https://www.twitch.tv/{}",
        },
        Platform {
            name: "Medium",
            profile_url: "https://medium.com/@{}",
        },
        Platform {
            name: "Steam",
            profile_url: "https://steamcommunity.com/id/{}",
        },
        Platform {
            name: "Keybase",
            profile_url: "https://keybase.io/{}",
        },
        Platform {
            name: "HackerNews",
            profile_url: "https://news.ycombinator.com/user?id={}",
        },
        Platform {
            name: "About.me",
            profile_url: "https://about.me/{}",
        },
    ]
}
