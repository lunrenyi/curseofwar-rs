# 开源项目SEO自救 | 为什么我开源的项目在Google上搜不到，怎么办？

最近，我在GitHub上开源了我的项目 lunrenyi/curseofwar（一个基于Rust重写的经典RTS游戏）。

出于好奇，我在Google上搜了一下相关的关键词，结果全是大佬的项目排在首页，自己的连个影子都没有。

· 搜索 site:github.com/lunrenyi：只出来了我的个人主页，Google对我的定位完全没有和游戏、curseofwar产生关联。
· 搜索 site:github.com curseofwar 或 curseofwar rust：首页全被原版（a-nikolaev/curseofwar）、老外的Rust移植版（DM-Earth/curseofrust）和Spicy Lobster项目霸占。

为什么我的项目完全没有出现在搜索结果中？

1. 尚未被Google收录。我在Google搜索 site:github.com/lunrenyi/curseofwar ，结果为0。我猜测这是因为项目的README不够完善，Google根本没抓取它。

2. 权威性不足和缺乏反向链接。原版 a-nikolaev/curseofwar 有十几年的历史，积累了大量的Stars和外部博客引用（外链）。老外的 DM-Earth/curseofrust 也有社区积累的权重。相比之下，我的项目在Google眼里缺乏信任度。

3. 同质化内容过滤（重复内容惩罚）。可能是代码高度相似，并且README中没有独特的描述。Google算法判定我的项目是重复内容，为了用户体验，它会优先展示原版或权重更高的移植版。

4. 跟我账号的关联断裂。虽然搜到了我的主页，但主页没有在显眼位置提供 curseofwar-rs 项目的链接，导致Google无法将我这个实体与“curseofwar”关键词建立强关联。

5. 缺乏站内SEO优化。搜 curseofwar rust 时，排名第一的项目标题包含了 [MIRROR] Curseofwar ported to Rust 等高匹配度关键词。而我的仓库只是介绍 curseofwar，并且没有设置好GitHub Topics，Google很难把它匹配给搜索意图。

为了完成我的收录和提升权重，我打算做以下实验：

第一步：完善仓库基础配置

· 在仓库右侧添加详细的 Description。
· 添加精准的 Topics（如 rust, curseofwar, real-time-strategy, game）。

第二步：优化README

· 增加一些独特的描述。比如：“基于原版Curse of War重写的Rust版本，保留了XX特性，优化了XX性能”。

第三步：建立内部链接（内链网）

· 在我的github主页的README中，将 curseofwar-rs 项目Pinned（置顶）。
· 在主页README中用Markdown链接指向该项目，写上“My Rust implementation of Curse of War”。

第四步：获取外部链接（外链）

去这些地方发帖介绍我的项目：
· Reddit：r/rust, r/gamedev,r/linux_gaming
· Hacker News, X (Twitter)
· 国内的微信公众号、知乎、V2EX、开源中国
  当有其他网站链接到我的GitHub仓库时，Google爬虫就会顺着链接来抓取并给予权重。

第五步：主动提交索引

· 在GitHub上发布一个 Release 版本，新版本发布一般会触发搜索引擎重新抓取。
· 在 Google Search Console (GSC) 中提交我的GitHub仓库URL请求收录。
· 注：你无法验证 github.com 域名的所有权，但可以通过GSC的“URL检查工具”要求Google重新抓取该特定URL。

