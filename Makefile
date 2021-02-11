all: data data/segment.srx data/nnsplit/en/model.onnx

data:
	mkdir data

data/segment.srx: data
	curl -o data/segment.srx https://github.com/languagetool-org/languagetool/raw/master/languagetool-core/src/main/resources/org/languagetool/resource/segment.srx

data/nnsplit/en/model.onnx:
	mkdir -p data/nnsplit/en
	curl -o data/nnsplit/en/model.onnx https://github.com/bminixhofer/nnsplit/raw/master/models/en/model.onnx


