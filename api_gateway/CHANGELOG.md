# Changelog

## 0.1.0 (2025-12-04)


### Features

* add mfa bypass based on env var (for load testing) ([7ff076c](https://github.com/louis-thevenet/brokerX/commit/7ff076c8381c63d19ecfb49d1f444d352b02b6bd))
* add order exec service ([b122786](https://github.com/louis-thevenet/brokerX/commit/b122786871261ab02f1a8e8c9826ea858189c880))
* add rejection reason to order db ([47c4a6a](https://github.com/louis-thevenet/brokerX/commit/47c4a6a808099d9521f43ee41592cebbbd66a7eb))
* add system metrics to all services and promotheus/grafana integration ([8914d05](https://github.com/louis-thevenet/brokerX/commit/8914d053dfef5ba9a4fea6a4198942108230e139))
* migrate to restapi and microservices ([#3](https://github.com/louis-thevenet/brokerX/issues/3)) ([3f3d70b](https://github.com/louis-thevenet/brokerX/commit/3f3d70b0ac487beeb9f7ad4beda47d9498c71275))
* replace order orchestrator with saga choregraphy  ([#5](https://github.com/louis-thevenet/brokerX/issues/5)) ([4f47457](https://github.com/louis-thevenet/brokerX/commit/4f474570521c8005d2bceb0b2b1e993ce1325718))
* setup prometheus ([6ac985c](https://github.com/louis-thevenet/brokerX/commit/6ac985ca06530ca097cc01be47a90641f3e3d6d5))


### Bug Fixes

* clippy lints ([ac86cb1](https://github.com/louis-thevenet/brokerX/commit/ac86cb1249bf42ed276d3485167de42c95da250c))
* don't call orchestrator in api gateway when creating orders ([dc215fb](https://github.com/louis-thevenet/brokerX/commit/dc215fb34e1e9742d28697ef0a53c1af766952be))
* make clippy happy ([3f3d70b](https://github.com/louis-thevenet/brokerX/commit/3f3d70b0ac487beeb9f7ad4beda47d9498c71275))
* minor optimizations ([d97e6b3](https://github.com/louis-thevenet/brokerX/commit/d97e6b3a17d6bbabadc2fe7b26c12fcdce07a215))
* optimize user service ([f7ddc87](https://github.com/louis-thevenet/brokerX/commit/f7ddc87d05ca72bb171e734d779967982abc2323))
* update holdings after funds operation ([efaea37](https://github.com/louis-thevenet/brokerX/commit/efaea37bf28a166c4909cd554e3f66e97c240f0c))
