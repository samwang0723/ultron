.PHONY: help test lint changelog-gen changelog-commit docker-build

help: ## show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {sub("\\\\n",sprintf("\n%22c"," "), $$2);printf "\033[36m%-25s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

PROJECT_NAME?=core
APP_NAME?=ultron
VERSION?=v0.2.0

SHELL = /bin/bash

########
# test #
########

test:
	cargo test --features testing

########
# lint #
########

lint: ## lints the entire codebase
	cargo clippy
	cargo fmt
	cargo check

#########
# build #
#########

docker-build: lint test docker-m1 ## build docker image in M1 device
	@printf "\nyou can now deploy to your env of choice:\ncd deploy\nENV=dev make deploy-latest\n"

docker-m1:
	@echo "[docker build] build local docker image on Mac M1"
	@docker build \
		-t samwang0723/$(APP_NAME):$(VERSION) \
		--build-arg LAST_MAIN_COMMIT_HASH=$(LAST_MAIN_COMMIT_HASH) \
		--build-arg LAST_MAIN_COMMIT_TIME=$(LAST_MAIN_COMMIT_TIME) \
		-f build/Dockerfile .

docker-amd64-deps:
	@echo "[docker buildx] install buildx depedency"
	@docker buildx create --name m1-builder
	@docker buildx use m1-builder
	@docker buildx inspect --bootstrap

docker-amd64:
	@echo "[docker buildx] build amd64 version docker image for Ubuntu AWS EC2 instance"
	@docker buildx use m1-builder
	@docker buildx build \
		--load --platform=linux/amd64 \
		-t samwang0723/$(APP_NAME):$(VERSION) \
		--build-arg LAST_MAIN_COMMIT_HASH=$(LAST_MAIN_COMMIT_HASH) \
		--build-arg LAST_MAIN_COMMIT_TIME=$(LAST_MAIN_COMMIT_TIME) \
		-f build/Dockerfile .

###########
# release #
###########

release: changelog-gen changelog-commit deploy-dev gh-release ## create a new tag to release this module

CAL_VER := v$(shell date "+%Y.%m.%d.%H%M")
PRODUCTION_YAML = deploy/production/kustomization.yaml
STAGING_YAML = deploy/staging/kustomization.yaml
DEV_YAML = deploy/develop/kustomization.yaml

deploy-dev:
	sed -i '' "s/newTag:.*/newTag: $(CAL_VER)/" $(DEV_YAML)
	git commit -S -m "ci: deploy tag $(CAL_VER) to adev" $(DEV_YAML)
	git tag $(CAL_VER)
	git push --atomic origin $(CAL_VER)

deploy-staging: ## deploy to staging env with a release tag
	@( \
	printf "Select a tag to deploy to staging:\n"; \
	select tag in `git tag --sort=-committerdate | head -n 10` ; do	\
		sed -i '' "s/newTag:.*/newTag: $$tag/" $(STAGING_YAML); \
		git commit -S -m "ci: deploy tag $$tag to staging" $(STAGING_YAML); \
		git push origin main; \
		break; \
	done )

deploy-production: confirm_deployment ## deploy to production env with a release tag
	@( \
	printf "Select a tag to deploy to production:\n"; \
	select tag in `git tag --sort=-committerdate | head -n 10` ; do	\
		sed -i '' "s/newTag:.*/newTag: $$tag/" $(PRODUCTION_YAML); \
		git commit -S -m "ci: deploy tag $$tag to production" $(PRODUCTION_YAML); \
		git push origin main; \
		break; \
	done )

confirm_deployment:
	@echo -n "Are you sure to deploy in production env? [y/N] " && read ans && [ $${ans:-N} = y ]

gh-release:
	@( \
	TAG=`git tag --sort=-committerdate | head -1` && \
	git cliff --latest --date-order | gh release create $$TAG -F - \
	)

#############
# changelog #
#############

MOD_VERSION = $(shell git describe --abbrev=0 --tags `git rev-list --tags --max-count=1`)

MESSAGE_CHANGELOG_COMMIT="chore(changelog): update CHANGELOG.md for $(MOD_VERSION)"

changelog-gen: ## generates the changelog in CHANGELOG.md
	@git cliff -o ./CHANGELOG.md && \
	printf "\nchangelog generated!\n"
	git add CHANGELOG.md

changelog-commit:
	git commit -m $(MESSAGE_CHANGELOG_COMMIT) ./CHANGELOG.md
