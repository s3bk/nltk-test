import sys
import os
import json
import multiprocessing as mp


def ner_spacy():
    import spacy
    from flair.tokenization import SegtokSentenceSplitter

    nlp = spacy.load("en_core_web_sm")
    splitter = SegtokSentenceSplitter()

    def ner(paras):
        out = []
        for para in paras:
            para_out = []
            sentences = splitter.split(para)

            for sentence in sentences:
                text = sentence.to_original_text()
                if len(text) == 0:
                    continue

                doc = nlp(text)
                o2 = sentence.start_pos

                for entity in doc.ents:
                    o3 = o2 + entity.start_char
                    span = [o3, o3 + len(entity.text)]
                    label = entity.label_
                    if label in ("ORG, PERS"):
                        match = {
                            "span": span,
                            "label": label,
                            "text": entity.text
                        }
                        para_out.append(match)
            
            out.append(para_out)
        
        return out

    return ner

def ner_stanza():
    import stanza
    nlp = stanza.Pipeline(lang='en', processors='tokenize,ner', use_gpu=False)

    def ner(paras):
        out = []
        for para in paras:
            para_out = []
            doc = nlp(para)

            # Analyze syntax
            #print("Noun phrases:", [chunk.text for chunk in doc.noun_chunks])
            #print("Verbs:", [token.lemma_ for token in doc if token.pos_ == "VERB"])

            # Find named entities, phrases and concepts
            for entity in doc.ents:
                if entity.type in ("GPE", "ORG"):
                    match = {
                        "span": [entity.start_char, entity.end_char],
                        "label": entity.type,
                        "text": entity.text
                    }
                    para_out.append(match)
            out.append(para_out)
        
        return out

    return ner

def ner_nnsplit():
    from nnsplit import NNSplit
    splitter = NNSplit("/home/sebk/Rust/nnsplit/models/en/model")
    
    def ner(paras):
        for para in splitter.split(paras):
            for sent in para:
                print(">", sent)
    
    return ner

def ner_flair():
    import flair, torch
    from flair.data import Sentence
    from flair.models import SequenceTagger
    from flair.tokenization import SegtokSentenceSplitter

    #flair.device = torch.device("cpu")

    # make a sentence

    # load the NER tagger
    tagger = SequenceTagger.load('ner-fast')
    splitter = SegtokSentenceSplitter()

    def ner(paras):
        sentences = []
        out = list([] for p in paras)

        for i, para in enumerate(paras):
            sentences.extend(map(lambda s: (i, s), splitter.split(para)))

        tagger.predict(list(s for i, s in sentences))

        for i, sentence in sentences:
            o2 = sentence.start_pos

            print(sentence)
            for entity in sentence.get_spans("ner"):
                if entity.tag in ("ORG", "PER"):
                    match = {
                        "span": [o2 + entity.start_pos, o2 + entity.end_pos],
                        "label": entity.tag,
                        "text": entity.text
                    }
                    out[i].append(match)

        return out

    return ner


#framework = (ner_spacy(), "spacy_sm")
framework = (ner_stanza(), "stanza")
#framework = (ner_flair(), "flair")

def analyze(args):
    file, out = args
    ner, ner_name = framework
    print(file)
    text = open(file, "r").read()
    offset = 0
    matches = ner(list(text.split("\n\n")))
    
    json.dump(matches, open(out, "wt"), indent=0)

def main():
    ner, ner_name = framework

    q = []
    for root in sys.argv[1:]:
        for (base, _, files) in os.walk(root):
            for file in files:
                if file.endswith(".plain"):
                    out = base + "/" + file[:-6] + "." + ner_name + ".json"
                    q.append((base + "/" + file, out))
    
    if ner_name in ("spacy_sm",):
        pool = mp.Pool(mp.cpu_count())
        pool.map(analyze, q)
    else:
        for arg in q:
            analyze(arg)


if __name__ == "__main__":
    main()
