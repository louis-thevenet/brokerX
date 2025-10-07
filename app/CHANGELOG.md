# Changelog

## 0.1.0 (2025-10-07)


### Features

* add /order endpoint ([487b85c](https://github.com/louis-thevenet/brokerX/commit/487b85ce3c80ffd01a3411b859a3553699d73535))
* add account verification before creation ([90c9d31](https://github.com/louis-thevenet/brokerX/commit/90c9d3156a6c80a41303571b06c0e21dfb346fde))
* add basic web app with account authentication ([cb642ac](https://github.com/louis-thevenet/brokerX/commit/cb642ac94445fb146e214061a32e252c3860cfa6))
* add get, put for user/id, user/id/orders ([5d6af3c](https://github.com/louis-thevenet/brokerX/commit/5d6af3c50072ce2a2f6328a838e2e2d651d5a338))
* add proper logging ([026a608](https://github.com/louis-thevenet/brokerX/commit/026a60879cfb59212e3e74fb9973eafa58c6714b))
* add user portfolio ([41cdc85](https://github.com/louis-thevenet/brokerX/commit/41cdc85931bf077285e873997d7e6894c16945a8))
* add user's orders to dashboard and an orders page ([54a6c61](https://github.com/louis-thevenet/brokerX/commit/54a6c61064b4cd788227a46b029943d42d77bde5))
* also write logs to stdout ([fc28248](https://github.com/louis-thevenet/brokerX/commit/fc28248c7173dfb74e6d8c832b6b51141811f6f6))
* implement database adapter ([540002b](https://github.com/louis-thevenet/brokerX/commit/540002b0a9e6ab109f8ac72d7c12d3d907b09bdb))
* implement deposit page ([73a13b3](https://github.com/louis-thevenet/brokerX/commit/73a13b37e42431d13ec86784209083da5835c85f))
* implement JWT token ([0b3d67b](https://github.com/louis-thevenet/brokerX/commit/0b3d67b2985c57e4d5cfea5e24558be362393c0d))
* implement MFA ([2d3129c](https://github.com/louis-thevenet/brokerX/commit/2d3129c20bee9ae6879203508c51ff41565f4bff))
* improve data types ([0a7f6d5](https://github.com/louis-thevenet/brokerX/commit/0a7f6d5446f25cfc313a333c64a6e75dc3d85ecd))
* init api ([8f52d8d](https://github.com/louis-thevenet/brokerX/commit/8f52d8d49c5a6253e5266fee26ce708d9bc737d2))
* init debug account and order repos ([30c6c51](https://github.com/louis-thevenet/brokerX/commit/30c6c519e24178c72c20fb4b3de6d342e1b66c5e))
* make put idempotent and add post endpoint ([10c215d](https://github.com/louis-thevenet/brokerX/commit/10c215d5fcc91b02d92937da74a1bb85d0eb28d0))
* move order processing to a thread pool ([9d9db48](https://github.com/louis-thevenet/brokerX/commit/9d9db485ff79c21c9eb0b05866de63bbab3e16c7))
* place_order page ([e3ef6b2](https://github.com/louis-thevenet/brokerX/commit/e3ef6b21dba13656dc96dfe50d865a3960f23730))


### Bug Fixes

* get rid of duplicate user repo handler ([225c74c](https://github.com/louis-thevenet/brokerX/commit/225c74c683bcb233d579a80a825e556ff9454f69))
* resend_mfa missing parameter ([63c7a0c](https://github.com/louis-thevenet/brokerX/commit/63c7a0cc5982bcc3ae7ae714b07705548c41f8e9))
* unverified accounts gets verified if user tries to log in ([076b63f](https://github.com/louis-thevenet/brokerX/commit/076b63f61188705daf452872b44120e15b572bc9))

## 0.1.0 (2025-09-29)


### Features

* add account verification before creation ([90c9d31](https://github.com/louis-thevenet/brokerX/commit/90c9d3156a6c80a41303571b06c0e21dfb346fde))
* add basic web app with account authentication ([cb642ac](https://github.com/louis-thevenet/brokerX/commit/cb642ac94445fb146e214061a32e252c3860cfa6))
* add proper logging ([026a608](https://github.com/louis-thevenet/brokerX/commit/026a60879cfb59212e3e74fb9973eafa58c6714b))
* add user portfolio ([41cdc85](https://github.com/louis-thevenet/brokerX/commit/41cdc85931bf077285e873997d7e6894c16945a8))
* add user's orders to dashboard and an orders page ([54a6c61](https://github.com/louis-thevenet/brokerX/commit/54a6c61064b4cd788227a46b029943d42d77bde5))
* also write logs to stdout ([fc28248](https://github.com/louis-thevenet/brokerX/commit/fc28248c7173dfb74e6d8c832b6b51141811f6f6))
* implement database adapter ([540002b](https://github.com/louis-thevenet/brokerX/commit/540002b0a9e6ab109f8ac72d7c12d3d907b09bdb))
* implement deposit page ([73a13b3](https://github.com/louis-thevenet/brokerX/commit/73a13b37e42431d13ec86784209083da5835c85f))
* implement JWT token ([0b3d67b](https://github.com/louis-thevenet/brokerX/commit/0b3d67b2985c57e4d5cfea5e24558be362393c0d))
* implement MFA ([2d3129c](https://github.com/louis-thevenet/brokerX/commit/2d3129c20bee9ae6879203508c51ff41565f4bff))
* improve data types ([0a7f6d5](https://github.com/louis-thevenet/brokerX/commit/0a7f6d5446f25cfc313a333c64a6e75dc3d85ecd))
* init debug account and order repos ([30c6c51](https://github.com/louis-thevenet/brokerX/commit/30c6c519e24178c72c20fb4b3de6d342e1b66c5e))
* move order processing to a thread pool ([9d9db48](https://github.com/louis-thevenet/brokerX/commit/9d9db485ff79c21c9eb0b05866de63bbab3e16c7))
* place_order page ([e3ef6b2](https://github.com/louis-thevenet/brokerX/commit/e3ef6b21dba13656dc96dfe50d865a3960f23730))


### Bug Fixes

* get rid of duplicate user repo handler ([225c74c](https://github.com/louis-thevenet/brokerX/commit/225c74c683bcb233d579a80a825e556ff9454f69))
* resend_mfa missing parameter ([63c7a0c](https://github.com/louis-thevenet/brokerX/commit/63c7a0cc5982bcc3ae7ae714b07705548c41f8e9))
* unverified accounts gets verified if user tries to log in ([076b63f](https://github.com/louis-thevenet/brokerX/commit/076b63f61188705daf452872b44120e15b572bc9))
