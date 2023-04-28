; Test if a collection is empty
(defun empty? (a)
  (or (= a '()) (= a "")))

; Accessors for extra elements in a list
(defun second (x)
  (first (rest x)))

(defun third (x)
  (first (first (rest x))))

; Get the nth item in a list l
(defun nth (n l)
  (if (= n 0)
    (first l)
    (nth (- n 1) (rest l))))

; Regular functional operators map, filter, reduces
(defun map (f l)
  (if (empty? l)
    '()
    (cons (f (first l)) (map f (rest l)))))

(defun filter (f l)
  (if (empty? l)
    '()
    (if (f (first l))
      (cons (first l) (filter f (rest l)))
      (filter f (rest l)))))

(defun reduce (f l)
  (if (empty? (rest l))
    (first l)
    (f (first l) (reduce f (rest l)))))


; Test if a list contains an element s
(defun contains (s l)
  (if (empty? l)
    false
    (if (= s (first l))
      true
      (contains s (rest l)))))

; Take the first n items from list l
(defun take (n l)
  (if (= n 0)
    '()
    (cons (first l) (take (- n 1) (rest l)))))

; Drop the first n items from list l
(defun drop (n l)
  (if (= n 0)
    l
    (drop (- n 1) (rest l))))

; Find the index of an item in a list, or -1 if not found
(defun index-of (e l)
  (if (empty? l)
    -1
    (if (= (first l) e)
      0
      (let (i (index-of e (rest l)))
        (if (= i -1)
          -1
          (+ 1 i))))))

; append
(defun append (a b)
  (if (empty? a)
    (if (empty? b)
      '()
      (cons (first b) (append a (rest b))))
    (cons (first a) (append (rest a) b))))

(defun reverse (l)
  (reverse-h '() l))

(defun reverse-h (out in)
  (if (empty? in)
    out
    (reverse-h (cons (first in) out) (rest in))))

; Split a list by a delimiter d
(defun split (l d)
  (let (at (index-of d l))
    (if (= at -1)
      (if (empty? l)
        '()
        (list l))
      (let (right (split (drop (+ at 1) l) d))
        (if (empty? right)
          (take at l)
          (append (take at l) right))))))

(alias ~ /home/devin)
