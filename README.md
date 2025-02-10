# Image Backend Server of It's MyPic
![幹嘛](https://mygodata.0m0.uk/images/ave-1_25106.jpg)

## API Schemas
* Static Image `/${season}/${episode}/${frame}.${format}`
  * `season`: String (`mygo`, `ave`, `ave-mujica`) | u32 (`1`, `2`)
  * `episode`: String (`1-3`, `4`, `5`, ..., `13`)
  * `frame`: u32
  * `format`: String (`jpg`, `jpeg`, `png`, `webp`)
* Animated Image `/${season}/${episode}/${startFrame}-${endFrame}.${format}`
  * `season`: String (`mygo`, `ave`, `ave-mujica`) | u32 (`1`, `2`)
  * `episode`: String (`1-3`, `4`, `5`, ..., `13`)
  * `startFrame`: u32
  * `endFrame`: u32 (`endFrame` must be greater than `startFrame`)
  * `format`: String (`gif`)
    * `gif`: Limited in 3600 frames (150 seconds)
* (deprecated) Legacy Static Image `/${season}-${episode}_${frame}.${format}`
  * `season`: String (`mygo`, `ave`, `ave-mujica`)
  * `episode`: String (`1-3`, `4`, `5`, ..., `13`)
  * `frame`: u32
  * `format`: String (`jpg`, `jpeg`, `png`, `webp`)

## Roadmap
* [x] Improve Animated Image Performance
