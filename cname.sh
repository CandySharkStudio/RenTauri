#!/bin/bash

PARAMS=("projn" "title" "pkgn" "ver" "desc")

declare -A PROMPTS
PROMPTS[projn]="请输入游戏名（必须为英文字母，以短横线做分割单词！）："
PROMPTS[title]="请输入标题名（可以为任意字符！）："
PROMPTS[pkgn]="请输入游戏包名（打包 Android 时作区分包用！建议填写：com.<作者名>.<游戏名>）："
PROMPTS[ver]="请输入游戏版本号（例如 1.0.0）："
PROMPTS[desc]="请输入游戏描述（随便填啥都可以）："

declare -A VALUES

while [[ $# -gt 0 ]]; do
    case $1 in
        -projn|--project-name)
            VALUES[projn]="$2"
            shift 2
            ;;
        -title|--project-title)
            VALUES[title]="$2"
            shift 2
            ;;
        -pkgn|--package-name)
            VALUES[pkgn]="$2"
            shift 2
            ;;
        -ver|--version)
            VALUES[ver]="$2"
            shift 2
            ;;
        -desc|--description)
            VALUES[desc]="$2"
            shift 2
            ;;
        -*)
            echo "错误：未知参数 $1"
            exit 1
            ;;
        *)
            echo "错误：未知传入值 $1"
            exit 1
            ;;
    esac
done
echo "检测到你有参数未填，接下来进入交互模式填写！请记住，如果不按照要求填写很可能会导致打包失败！"
for param in "${PARAMS[@]}"; do
    if [[ -z "${VALUES[$param]}" ]]; then
        read -p "${PROMPTS[$param]}" VALUES[$param]
    fi
done

# 最终输出结果
echo "-------------------"
for param in "${PARAMS[@]}"; do
    echo "${param}: ${VALUES[$param]}"
done
echo "-------------------"
